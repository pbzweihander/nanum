mod app;
mod download;
mod navbar;
mod route;
mod types;
mod upload;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<app::App>::new().render();
}
