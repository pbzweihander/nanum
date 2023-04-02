use aead::stream::EncryptorBE32;
use chacha20poly1305::{
    aead::{generic_array::GenericArray, Aead},
    Key, KeyInit, XChaCha20Poly1305,
};
use futures_util::TryStreamExt;
use gloo_net::http::Request;
use hkdf::Hkdf;
use js_sys::Uint8Array;
use nanum_core::types::MetadataCreationReq;
use serde::Deserialize;
use sha2::Sha256;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, File, HtmlInputElement, SubmitEvent};
use yew::{function_component, html, use_effect_with_deps, use_state, Html, TargetCast};

use crate::{navbar::NavBar, types::User, BLOCK_SIZE};

#[derive(Deserialize)]
struct PostMetadataResp {
    id: String,
}

#[function_component(Upload)]
pub fn upload() -> Html {
    let user = use_state(String::new);

    use_effect_with_deps(
        {
            let user = user.clone();
            move |_| {
                spawn_local(async move {
                    let fetched_user: User = Request::get("/api/user")
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap();
                    user.set(fetched_user.primary_email);
                });
                || ()
            }
        },
        (),
    );

    let file = use_state::<Option<File>, _>(|| None);
    let passphrase = use_state(String::new);

    let on_file_change = {
        let file = file.clone();
        move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if let Some(f) = files.get(0) {
                    file.set(Some(f));
                }
            }
        }
    };
    let on_passphrase_change = {
        let passphrase = passphrase.clone();
        move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            passphrase.set(input.value());
        }
    };

    let onsubmit = {
        let file = file.clone();
        let passphrase = passphrase.clone();
        move |e: SubmitEvent| {
            e.prevent_default();

            if file.is_none() || passphrase.is_empty() {
                return;
            }

            let file = file.as_ref().unwrap();

            // Reference: https://github.com/skystar-p/hako/blob/main/webapp/src/upload.rs

            // generate salt for hkdf expand()
            let mut salt = [0u8; 32];
            if let Err(error) = getrandom::getrandom(&mut salt) {
                log::error!("cannot get random salt value: {:?}", error);
                return;
            }

            // generate key by hkdf
            let h = Hkdf::<Sha256>::new(Some(&salt), passphrase.as_bytes());
            let mut key_slice = [0u8; 32];
            if let Err(err) = h.expand(&[], &mut key_slice[..]) {
                log::error!("cannot expand passphrase by hkdf: {:?}", err);
                return;
            }

            // generate nonce for XChaCha20Poly1305
            let mut stream_nonce = [0u8; 19];
            if let Err(err) = getrandom::getrandom(&mut stream_nonce) {
                log::error!("cannot get random nonce value: {:?}", err);
                return;
            }
            let mut filename_nonce = [0u8; 24];
            if let Err(err) = getrandom::getrandom(&mut filename_nonce) {
                log::error!("cannot get random nonce value: {:?}", err);
                return;
            }

            let key = Key::from_slice(&key_slice);
            let cipher = XChaCha20Poly1305::new(key);

            let stream_nonce = GenericArray::from_slice(stream_nonce.as_ref());
            let filename_nonce = GenericArray::from_slice(filename_nonce.as_ref());

            let sys_stream = {
                if let Ok(s) = file.stream().dyn_into() {
                    s
                } else {
                    log::error!("file stream is not web_sys::ReadableStream");
                    return;
                }
            };

            // encrypt filename
            let filename = file.name();
            let encrypted_filename = {
                match cipher.encrypt(
                    filename_nonce,
                    filename.bytes().collect::<Vec<u8>>().as_ref(),
                ) {
                    Ok(encrypted) => encrypted,
                    Err(err) => {
                        log::error!("failed to encrypt filename: {:?}", err);
                        return;
                    }
                }
            };

            // read file
            let stream = wasm_streams::ReadableStream::from_raw(sys_stream).into_stream();

            // stream which read files and transforms that `Uint8Array`s to `Result<Vec<u8>>`.
            let fut = stream
                .and_then(|b| async move { b.dyn_into::<Uint8Array>() })
                .map_ok(|arr| arr.to_vec());

            let mut fut = Box::pin(fut);

            let metadata = MetadataCreationReq {
                salt: salt.to_vec(),
                nonce: stream_nonce.to_vec(),
                filename_nonce: filename_nonce.to_vec(),
                filename: encrypted_filename,
                size: file.size() as usize,
            };

            let stream_nonce = *stream_nonce;

            // core logic of streaming upload / encryption
            let encrypt_routine = async move {
                // use stream encryptor
                let mut encryptor = EncryptorBE32::from_aead(cipher, &stream_nonce);
                // send prepare request

                let req = match Request::post("/api/metadata").json(&metadata) {
                    Ok(req) => req,
                    Err(error) => {
                        log::error!("failed to make request: {:?}", error);
                        return;
                    }
                };
                let resp = match req.send().await {
                    Ok(resp) => resp,
                    Err(error) => {
                        log::error!("failed to upload metadata: {:?}", error);
                        return;
                    }
                };

                if resp.status() != 200 {
                    log::error!("failed to upload metadata. status code: {}", resp.status());
                    return;
                }

                let id = match resp.json::<PostMetadataResp>().await {
                    Ok(resp) => resp.id,
                    Err(error) => {
                        log::error!("failed to read response body: {:?}", error);
                        return;
                    }
                };

                let mut seq: i64 = 1;
                let mut buffer = Vec::<u8>::with_capacity(BLOCK_SIZE);
                // start encryption and upload
                while let Ok(Some(v)) = fut.try_next().await {
                    let mut v: &[u8] = v.as_ref();
                    // divide inputs into fixed block size
                    while buffer.len() + v.len() >= BLOCK_SIZE {
                        let split_idx = BLOCK_SIZE - buffer.len();
                        buffer.extend(&v[..split_idx]);
                        // upload chunk to server
                        // this will block next encryption...
                        // maybe there is more good way to handle this
                        let chunk = match encryptor.encrypt_next(buffer.as_ref()) {
                            Ok(chunk) => chunk,
                            Err(error) => {
                                log::error!("failed to encrypt chunk: {:?}", error);
                                return;
                            }
                        };
                        let chunk_len = chunk.len();

                        let chunk: Uint8Array = chunk.as_slice().into();
                        let resp = match Request::post(&format!("/api/file/{id}/{seq}"))
                            .body(chunk)
                            .send()
                            .await
                        {
                            Ok(resp) => resp,
                            Err(error) => {
                                log::error!("failed to upload chunk: {:?}", error);
                                return;
                            }
                        };

                        if resp.status() != 200 {
                            log::error!("failed to upload chunk. status code: {}", resp.status());
                            return;
                        }

                        buffer.clear();
                        v = &v[split_idx..];
                        seq += 1;

                        // TODO: Progress
                        log::info!("chunk_len: {}", chunk_len);
                    }
                    buffer.extend(v);
                }

                // upload last chunk
                let chunk = match encryptor.encrypt_last(buffer.as_ref()) {
                    Ok(chunk) => chunk,
                    Err(error) => {
                        log::error!("failed to encrypt chunk: {:?}", error);
                        return;
                    }
                };

                let chunk: Uint8Array = chunk.as_slice().into();
                let resp = match Request::post(&format!("/api/file/{id}/{seq}"))
                    .body(chunk)
                    .send()
                    .await
                {
                    Ok(resp) => resp,
                    Err(error) => {
                        log::error!("failed to upload chunk: {:?}", error);
                        return;
                    }
                };

                if resp.status() != 200 {
                    log::error!("failed to upload chunk. status code: {}", resp.status());
                    return;
                }

                // TODO: Complete
                log::info!("finished");
            };

            // spawn entire routine in promise
            // TODO: research Web Workers and try to gain more performance
            spawn_local(encrypt_routine);
        }
    };

    let is_submit_disabled = file.is_none() || passphrase.is_empty();

    html! {
        <NavBar user={(*user).clone()}>
            <div class="w-full flex justify-center mb-4">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-6 h-6">
                  <path fill-rule="evenodd" d="M10.5 3.75a6 6 0 00-5.98 6.496A5.25 5.25 0 006.75 20.25H18a4.5 4.5 0 002.206-8.423 3.75 3.75 0 00-4.133-4.303A6.001 6.001 0 0010.5 3.75zm2.03 5.47a.75.75 0 00-1.06 0l-3 3a.75.75 0 101.06 1.06l1.72-1.72v4.94a.75.75 0 001.5 0v-4.94l1.72 1.72a.75.75 0 101.06-1.06l-3-3z" clip-rule="evenodd" />
                </svg>
            </div>
            <form class="form-control w-full max-w-xs" {onsubmit}>
                <input type="file" class="file-input w-full mb-4" onchange={on_file_change} />
                <input
                    type="password"
                    placeholder="Passphrase"
                    class="input w-full mb-4"
                    onchange={on_passphrase_change}
                />
                <input type="submit" class="btn" value="Upload" disabled={is_submit_disabled} />
            </form>
        </NavBar>
    }
}
