mod app;
mod input;
mod render;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new().expect("failed to create event loop");
    let mut app = app::App::default();
    event_loop.run_app(&mut app).expect("event loop error");
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).ok();

    wasm_bindgen_futures::spawn_local(async {
        let event_loop = winit::event_loop::EventLoop::new().expect("failed to create event loop");
        let mut app = app::App::default();
        event_loop.run_app(&mut app).expect("event loop error");
    });
}
