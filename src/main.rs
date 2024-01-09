use anyhow::{anyhow, Result};
use ash::Entry;
use glfw::fail_on_errors;
use rusty_games::{create_graphics_pipeline, create_logical_device, init_logging};

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

    let entry = Entry::linked();
    let logical_device = create_logical_device(&entry, &glfw, &window)?;
    create_graphics_pipeline(&entry, &window, &logical_device)?;

    while !window.should_close() {
        glfw.wait_events();
    }

    Ok(())
}
