use aead::{generic_array::GenericArray, stream::DecryptorBE32, Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305};
use gloo_net::http::Request;
use hkdf::Hkdf;
use js_sys::{Array, Uint8Array};
use nanum_core::types::Metadata;
use sha2::Sha256;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlInputElement, HtmlLinkElement, SubmitEvent, Url};
use yew::{
    function_component, html, use_callback, use_effect_with_deps, use_node_ref, use_state, Html,
    NodeRef, Properties, TargetCast, UseStateHandle,
};

use crate::navbar::NavBar;

#[derive(Properties, PartialEq)]
pub struct DownloadProps {
    pub id: String,
}

#[function_component(Download)]
pub fn download(props: &DownloadProps) -> Html {
    let metadata = use_state::<Option<Metadata>, _>(|| None);

    use_effect_with_deps(
        {
            let id = props.id.clone();
            let metadata = metadata.clone();
            move |_| {
                spawn_local(async move {
                    let resp = match Request::get(&format!("/api/metadata/{id}")).send().await {
                        Ok(resp) => resp,
                        Err(error) => {
                            log::error!("failed to fetch metadata: {:?}", error);
                            return;
                        }
                    };
                    if resp.status() != 200 {
                        log::error!("failed to fetch metadata. status code: {}", resp.status());
                        return;
                    }
                    let fetched_metadata: Metadata = match resp.json().await {
                        Ok(resp) => resp,
                        Err(error) => {
                            log::error!("failed to read metadata response: {:?}", error);
                            return;
                        }
                    };
                    metadata.set(Some(fetched_metadata));
                });
                || ()
            }
        },
        (),
    );

    let a_ref = use_node_ref();

    let passphrase = use_state(String::new);

    let download_started = use_state(|| false);
    let decrypted_filename = use_state::<Option<String>, _>(|| None);
    let progress = use_state(|| 0usize);

    let on_passphrase_change = use_callback(
        move |e: Event, passphrase| {
            let input: HtmlInputElement = e.target_unchecked_into();
            passphrase.set(input.value());
        },
        passphrase.clone(),
    );

    let onsubmit = use_callback::<
        _,
        _,
        _,
        (
            UseStateHandle<String>,
            UseStateHandle<bool>,
            UseStateHandle<Option<String>>,
            UseStateHandle<usize>,
            NodeRef,
        ),
    >(
        {
            let id = props.id.clone();
            let metadata = metadata.clone();
            move |e: SubmitEvent,
                  (passphrase, download_started, decrypted_filename_state, progress, a_ref)
            | {
                e.prevent_default();

                if **download_started || passphrase.is_empty() || metadata.is_none() {
                    return;
                }

                download_started.set(true);

                let metadata = metadata.as_ref().unwrap();

                // Reference: https://github.com/skystar-p/hako/blob/main/webapp/src/download.rs

                // decrypt filename first
                // restore key from passphrase
                let h = Hkdf::<Sha256>::new(Some(metadata.salt.as_ref()), passphrase.as_bytes());
                let mut key_slice = [0u8; 32];
                if let Err(err) = h.expand(&[], &mut key_slice[..]) {
                    log::error!("cannot expand passphrase by hkdf: {:?}", err);
                    return;
                }
                let key = Key::clone_from_slice(&key_slice);
                let cipher = XChaCha20Poly1305::new(&key);
                let filename_nonce = GenericArray::from_slice(metadata.filename_nonce.as_ref());
                let decrypted_filename = {
                    match cipher.decrypt(filename_nonce, metadata.filename.as_ref()) {
                        Ok(decrypted) => decrypted,
                        Err(err) => {
                            log::error!("failed to decrypt filename: {:?}", err);
                            return;
                        }
                    }
                };

                decrypted_filename_state.set(Some(
                    String::from_utf8_lossy(&decrypted_filename).to_string(),
                ));

                let seq_count = (metadata.size as f64 / metadata.block_size as f64).ceil() as usize;

                let id = id.clone();
                let metadata = metadata.clone();
                let progress = progress.clone();
                let a_ref = a_ref.clone();
                spawn_local(async move {
                    // make cipher
                    let cipher = XChaCha20Poly1305::new(&key);
                    let stream_nonce = GenericArray::from_slice(metadata.nonce.as_ref());
                    let mut decryptor = DecryptorBE32::from_aead(cipher, stream_nonce);

                    // preallocate buffers
                    let mut body = Vec::<u8>::with_capacity(metadata.size);

                    for seq in 1..=(seq_count - 1) {
                        let resp = match Request::get(&format!("/api/file/{id}/{seq}")).send().await
                        {
                            Ok(resp) => resp,
                            Err(error) => {
                                log::error!("failed to fetch chunk: {:?}", error);
                                return;
                            }
                        };
                        if resp.status() != 200 {
                            log::error!("failed to fetch chunk. status code: {}", resp.status());
                            return;
                        }
                        let chunk = match resp.binary().await {
                            Ok(resp) => resp,
                            Err(error) => {
                                log::error!("failed to read chunk response: {:?}", error);
                                return;
                            }
                        };

                        let mut res = match decryptor.decrypt_next(chunk.as_slice()) {
                            Ok(res) => res,
                            Err(error) => {
                                log::error!("failed to decrypt chunk: {:?}", error);
                                return;
                            }
                        };

                        body.append(&mut res);
                        progress.set(body.len());
                    }

                    let resp = match Request::get(&format!("/api/file/{id}/{seq_count}"))
                        .send()
                        .await
                    {
                        Ok(resp) => resp,
                        Err(error) => {
                            log::error!("failed to fetch chunk: {:?}", error);
                            return;
                        }
                    };
                    if resp.status() != 200 {
                        log::error!("failed to fetch chunk. status code: {}", resp.status());
                        return;
                    }
                    let chunk = match resp.binary().await {
                        Ok(resp) => resp,
                        Err(error) => {
                            log::error!("failed to read chunk response: {:?}", error);
                            return;
                        }
                    };

                    let mut res = match decryptor.decrypt_last(chunk.as_slice()) {
                        Ok(res) => res,
                        Err(error) => {
                            log::error!("failed to decrypt chunk: {:?}", error);
                            return;
                        }
                    };

                    body.append(&mut res);
                    progress.set(body.len());

                    if body.len() != metadata.size {
                        log::error!(
                            "received bytes does not match expected size. expected: {}, actual: {}",
                            metadata.size,
                            body.len()
                        );
                        return;
                    }

                    let a = match a_ref.cast::<HtmlLinkElement>() {
                        Some(a) => a,
                        None => {
                            log::error!("failed to get a ref");
                            return;
                        }
                    };

                    // Touching filesystem in browser is strictly prohibited because of security
                    // context, so we cannot pipe Vec<u8> into file directly. In order to invoke file
                    // download for user, we have to convert it into `Blob` object and retrieve its
                    // object url(which will resides in memory).
                    // But we cannot use Vec<u8>'s reference directly because `Blob` is immutable
                    // itself, so we have to full-copy the whole buffer. Not efficient of course...
                    // In addition, moving WASM's linear memory into JS's `Uint8Array` also cause full
                    // copy of buffer, which is worse... (consumes `file_size` * 3 amount of memory)
                    // So in here, we use unsafe method `Uint8Array::view()` which just unsafely map
                    // WASM's memory into linear `Uint8Array`'s memory representation, which will not
                    // cause copy of memory. `mem_view` and decrypted content should have same
                    // lifetime, and those should not be reallocated.
                    unsafe {
                        let blob_parts = Array::new();
                        let mem_view = Uint8Array::view(&body);
                        blob_parts.push(&mem_view);
                        let decrypted_blob = {
                            // causes full copy of buffer. this will consumes lots of memory, but there
                            // are no workaround currently.
                            match web_sys::Blob::new_with_u8_array_sequence(&blob_parts) {
                                Ok(blob) => blob,
                                Err(err) => {
                                    log::error!("failed to make data into blob: {:?}", err);
                                    return;
                                }
                            }
                        };
                        let obj_url = {
                            match Url::create_object_url_with_blob(&decrypted_blob) {
                                Ok(u) => u,
                                Err(err) => {
                                    log::error!("failed to make blob into object url: {:?}", err);
                                    return;
                                }
                            }
                        };

                        a.set_href(&obj_url);
                        // invoke download action
                        a.click();

                        // immediately revoke object url so that memory consumed by `Blob` object will
                        // soon released by GC.
                        if let Err(e) = Url::revoke_object_url(&obj_url) {
                            log::error!("failed to revoke object url: {:?}", e);
                        }
                    }
                });
            }
        },
        (
            passphrase,
            download_started.clone(),
            decrypted_filename.clone(),
            progress.clone(),
            a_ref.clone(),
        ),
    );

    let progress_show = if let Some(metadata) = &*metadata {
        let p = (*progress as f64) / (metadata.size as f64) * 1000.;
        html! {
            <div class="w-full mt-4">
                <progress class="progress w-full" value={format!("{}", p)} max="1000" />
            </div>
        }
    } else {
        html! { <></> }
    };
    let decrypted_filename_show = if let Some(filename) = &*decrypted_filename {
        html! {
            <div class="w-full mt-4">{filename}</div>
        }
    } else {
        html! { <></> }
    };

    html! {
        <NavBar>
            <div class="max-w-xs">
                <div class="w-full flex justify-center mb-4">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-6 h-6">
                      <path fill-rule="evenodd" d="M10.5 3.75a6 6 0 00-5.98 6.496A5.25 5.25 0 006.75 20.25H18a4.5 4.5 0 002.206-8.423 3.75 3.75 0 00-4.133-4.303A6.001 6.001 0 0010.5 3.75zm2.25 6a.75.75 0 00-1.5 0v4.94l-1.72-1.72a.75.75 0 00-1.06 1.06l3 3a.75.75 0 001.06 0l3-3a.75.75 0 10-1.06-1.06l-1.72 1.72V9.75z" clip-rule="evenodd" />
                    </svg>
                </div>
                <form class="form-control w-full" {onsubmit}>
                    <label class="label label-text">{"Passphrase"}</label>
                    <input
                        type="password"
                        class="input input-bordered w-full"
                        onchange={on_passphrase_change}
                    />
                    if !*download_started {
                        <input type="submit" class="btn mt-4" value="Download" />
                    }
                </form>
                {progress_show}
                {decrypted_filename_show}
            </div>
            <a download={(*decrypted_filename).clone()} class="hidden" ref={a_ref.clone()}></a>
        </NavBar>
    }
}
