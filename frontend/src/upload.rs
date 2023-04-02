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
