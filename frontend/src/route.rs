use yew::{html, Html};
use yew_router::Routable;

use crate::{download::Download, upload::Upload};

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[at("/")]
    Upload,
    #[at("/:id")]
    Download { id: String },
}

pub fn switch(route: Route) -> Html {
    match route {
        Route::Upload => html! { <Upload /> },
        Route::Download { id } => html! { <Download {id} /> },
    }
}
