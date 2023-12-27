mod physical_device;

use ash::vk::{PhysicalDeviceType, QueueFlags};

use crate::vulkan::VkInstanceGuard;

pub use self::physical_device::PhysicalDevice;

/// Wraps the Vulkan APIs to interact with physical devices
pub struct PhysicalDeviceManager<'instance> {
    instance_guard: &'instance VkInstanceGuard,
}

impl<'instance> PhysicalDeviceManager<'instance> {
    /// Creates a new PhysicalDeviceManager for the given instance.
    pub fn new(instance_guard: &'instance VkInstanceGuard) -> Self {
        Self { instance_guard }
    }

    /// Queries the physical devices available on this machine and returns them
    /// in order of preference.
    pub fn query_physical_devices(&self) -> anyhow::Result<Vec<PhysicalDevice>> {
        let mut physical_devices =
            unsafe { self.instance_guard.instance.enumerate_physical_devices()? }
                .into_iter()
                .map(|pd| PhysicalDevice::new(self.instance_guard, pd))
                .filter(|pd| pd.queue_family_supports(QueueFlags::GRAPHICS))
                .collect::<Vec<_>>();
        physical_devices.sort_by_cached_key(|physical_device| {
            // higher is better
            let mut score = match physical_device.props.device_type {
                PhysicalDeviceType::DISCRETE_GPU => 100,
                PhysicalDeviceType::INTEGRATED_GPU => 10,
                _ => 1,
            };
            // check if we have dedicated transfer queues
            if physical_device.queue_family_supports(QueueFlags::TRANSFER) {
                score += 1;
            }
            score
        });
        // sort descending
        physical_devices.reverse();
        Ok(physical_devices)
    }
}
