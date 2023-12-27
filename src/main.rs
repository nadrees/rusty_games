use rusty_games::{init_logging, VulkanManager, WindowManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let vulkan_manager = VulkanManager::try_new()?;
    let mut window_manager = WindowManager::try_new()?;

    window_manager.run_event_loop();

    Ok(())
}
