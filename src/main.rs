use anyhow::{anyhow, Result};
use glfw::fail_on_errors;
use rusty_games::{create_graphics_engine, init_logging};
use tracing::info;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_TITLE: &str = "Hello, Triangle";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let mut glfw = glfw::init(fail_on_errors!())?;
    glfw.window_hint(glfw::WindowHint::Visible(true));
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
    glfw.window_hint(glfw::WindowHint::Resizable(false));
    let (window, _) = glfw
        .create_window(
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            WINDOW_TITLE,
            glfw::WindowMode::Windowed,
        )
        .ok_or(anyhow!("Failed to create window"))?;

    let graphics_engine = create_graphics_engine(&glfw, &window)?;

    while !window.should_close() {
        glfw.poll_events();
        graphics_engine.render_frame()?;
    }

    info!("Window closed, shutting down");

    Ok(())
}
