use rusty_games::{VulkanManager, WindowManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vulkan_manager = VulkanManager::try_new()?;
    let mut window_manager = WindowManager::try_new()?;

    window_manager.run_event_loop();

    Ok(())
}
