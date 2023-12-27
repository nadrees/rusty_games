mod extensions_registry;
mod instance_guard;
mod layers_registry;
mod logical_device;
mod physical_device_manager;
mod surface_guard;

use anyhow::Result;
use ash::Entry;
use glfw::PWindow;
use instance_guard::VkInstanceGuard;
use physical_device_manager::PhysicalDeviceManager;

pub struct VulkanManager {
    _instance: VkInstanceGuard,
}

impl VulkanManager {
    pub fn try_new(window: &PWindow, additional_extensions: Option<Vec<String>>) -> Result<Self> {
        let entry = Entry::linked();
        let mut instance = VkInstanceGuard::try_new(&entry, window, additional_extensions)?;

        let physical_device_manager = PhysicalDeviceManager::new(&instance);
        let physical_devices = physical_device_manager.query_physical_devices()?;
        let logical_device = instance.create_logical_device(physical_devices.first().unwrap())?;

        Ok(Self {
            _instance: instance,
        })
    }
}
