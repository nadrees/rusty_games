use ash::vk::{PhysicalDeviceProperties, QueueFamilyProperties, QueueFlags};

use crate::vulkan::VkInstanceGuard;

pub struct PhysicalDevice {
    pub physical_device: ash::vk::PhysicalDevice,
    pub props: PhysicalDeviceProperties,
    queue_family_props: Vec<QueueFamilyProperties>,
}

impl PhysicalDevice {
    pub fn new(instance_guard: &VkInstanceGuard, physical_device: ash::vk::PhysicalDevice) -> Self {
        let props = unsafe {
            instance_guard
                .instance
                .get_physical_device_properties(physical_device)
        };
        let queue_family_props = unsafe {
            instance_guard
                .instance
                .get_physical_device_queue_family_properties(physical_device)
        };

        Self {
            physical_device,
            props,
            queue_family_props,
        }
    }

    pub fn queue_family_supports(&self, flag: QueueFlags) -> bool {
        self.queue_family_props
            .iter()
            .any(|qfp| qfp.queue_flags.contains(flag))
    }

    pub fn queue_family_index_for(&self, flag: QueueFlags) -> Option<usize> {
        for (index, queue_family_props) in self.queue_family_props.iter().enumerate() {
            if queue_family_props.queue_flags.contains(flag) {
                return Some(index);
            }
        }
        return None;
    }
}
