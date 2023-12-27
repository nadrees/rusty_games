use glfw::fail_on_errors;
use rusty_games::{init_logging, VulkanManager, WindowManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let mut glfw = glfw::init(fail_on_errors!())?;
    glfw.window_hint(glfw::WindowHint::Visible(true));
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let mut window_manager = WindowManager::try_new(&mut glfw)?;
    let vulkan_manager = VulkanManager::try_new(
        &window_manager.window,
        glfw.get_required_instance_extensions(),
    )?;

    window_manager.run_event_loop(&mut glfw);

    Ok(())
}
