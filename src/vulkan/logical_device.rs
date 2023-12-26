use ash::{
    vk::{DeviceCreateInfo, DeviceQueueCreateInfo, QueueFlags},
    Device,
};

use crate::VkInstanceGuard;

use super::physical_device_manager::PhysicalDevice;

pub struct LogicalDevice {
    device: Device,
}

impl LogicalDevice {
    pub fn try_new(
        instance: &VkInstanceGuard,
        physical_device: &PhysicalDevice,
    ) -> anyhow::Result<Self> {
        let priorities = [1.0f32];

        let graphics_queue_index = physical_device
            .queue_family_index_for(QueueFlags::GRAPHICS)
            .unwrap();
        let transfer_queue_index = physical_device.queue_family_index_for(QueueFlags::TRANSFER);

        let mut queue_infos = vec![DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index as u32)
            .queue_priorities(&priorities)
            .build()];
        if let Some(transfer_queue_index) = transfer_queue_index {
            if transfer_queue_index != graphics_queue_index {
                queue_infos.push(
                    DeviceQueueCreateInfo::builder()
                        .queue_family_index(transfer_queue_index as u32)
                        .queue_priorities(&priorities)
                        .build(),
                );
            }
        }

        let device_create_info = DeviceCreateInfo::builder().queue_create_infos(&queue_infos);

        let device = unsafe {
            instance.instance.create_device(
                physical_device.physical_device,
                &device_create_info,
                None,
            )?
        };

        Ok(Self { device })
    }
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None) };
    }
}
