use yew::{function_component, html, Html};

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <h1 class="text-xl underline">{ "Hello, world!" }</h1>
    }
}
