use ash::Entry;
use rusty_games::{PhysicalDeviceManager, VkInstanceGuard};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::linked();
    let mut instance = VkInstanceGuard::try_new(&entry)?;
    let physical_device_manager = PhysicalDeviceManager::new(&instance);
    let physical_devices = physical_device_manager.query_physical_devices()?;
    let logical_device = instance.create_logical_device(physical_devices.first().unwrap())?;
    Ok(())
}
