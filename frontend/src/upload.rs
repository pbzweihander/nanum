use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, File, HtmlInputElement, SubmitEvent};
use yew::{function_component, html, use_effect_with_deps, use_state, Html, TargetCast};

use crate::{navbar::NavBar, types::User};

#[function_component(Upload)]
pub fn upload() -> Html {
    let user = use_state(String::new);

    use_effect_with_deps(
        {
            let user = user.clone();
            move |_| {
                let user = user.clone();
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

    let onsubmit = {
        let file = file.clone();
        let passphrase = passphrase.clone();
        move |e: SubmitEvent| {
            e.prevent_default();

            if file.is_some() && !passphrase.is_empty() {
                log::info!("{:?} {}", *file, *passphrase);
            }
        }
    };
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
