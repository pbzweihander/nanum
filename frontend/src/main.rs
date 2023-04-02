mod app;
mod download;
mod navbar;
mod route;
mod types;
mod upload;

// const BLOCK_SIZE: usize = 1024 * 1024 * 10;
const BLOCK_SIZE: usize = 1024 * 1024;
const BLOCK_OVERHEAD: usize = 16;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<app::App>::new().render();
}
