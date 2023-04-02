use yew::{function_component, html, Html, Properties};

use crate::navbar::NavBar;

#[derive(Properties, PartialEq)]
pub struct DownloadProps {
    pub id: String,
}

#[function_component(Download)]
pub fn download(props: &DownloadProps) -> Html {
    html! {
        <NavBar>
            <span>{ &props.id }</span>
        </NavBar>
    }
}
