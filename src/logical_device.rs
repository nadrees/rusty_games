use std::{collections::HashSet, ops::Deref, rc::Rc};

use anyhow::ensure;
use ash::{
    vk::{DeviceCreateInfo, DeviceQueueCreateInfo, PhysicalDeviceFeatures, Queue},
    Device,
};

use crate::{
    physical_device_surface::QueueFamilyIndicies, Instance, PhysicalDeviceSurface, Surface,
    SwapChainSupportDetails, REQUIRED_DEVICE_EXTENSIONS,
};

pub struct LogicalDevice {
    _instance: Rc<Instance>,
    device: Device,
    queue_handles: QueueHandles,
    physical_device_surface: PhysicalDeviceSurface,
}

impl LogicalDevice {
    pub fn get_queues(&self) -> &QueueHandles {
        &self.queue_handles
    }

    pub fn get_surface(&self) -> &Rc<Surface> {
        self.physical_device_surface.get_surface()
    }

    pub fn get_queue_family_indicies(&self) -> &QueueFamilyIndicies {
        self.physical_device_surface.get_queue_family_indicies()
    }

    pub fn get_swapchain_support_details(&self) -> &SwapChainSupportDetails {
        self.physical_device_surface.get_swapchain_support_details()
    }
}

impl TryFrom<PhysicalDeviceSurface> for LogicalDevice {
    type Error = anyhow::Error;

    fn try_from(physical_device_surface: PhysicalDeviceSurface) -> Result<Self, Self::Error> {
        let indicies = physical_device_surface.get_queue_family_indicies();
        ensure!(indicies.is_complete());

        let unique_queue_family_indicies = HashSet::from([
            indicies.graphics_family.unwrap() as u32,
            indicies.present_family.unwrap() as u32,
        ]);

        let queue_priorities = [1.0f32];
        let device_queue_creation_infos = unique_queue_family_indicies
            .into_iter()
            .map(|queue_family_index| {
                DeviceQueueCreateInfo::default()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&queue_priorities)
            })
            .collect::<Vec<_>>();

        let physical_device_features = PhysicalDeviceFeatures::default();

        let extension_names = REQUIRED_DEVICE_EXTENSIONS
            .iter()
            .map(|extension_name| (**extension_name).as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = DeviceCreateInfo::default()
            .queue_create_infos(&device_queue_creation_infos)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&extension_names);

        let logical_device = unsafe {
            physical_device_surface.instance.create_device(
                physical_device_surface.get_physical_device(),
                &device_create_info,
                None,
            )
        }?;

        let graphics_queue_handle =
            unsafe { logical_device.get_device_queue(indicies.graphics_family.unwrap() as u32, 0) };
        let present_queue_handle =
            unsafe { logical_device.get_device_queue(indicies.present_family.unwrap() as u32, 0) };
        let queue_handles = QueueHandles {
            graphics: graphics_queue_handle,
            present: present_queue_handle,
        };

        let instance = Rc::clone(&physical_device_surface.instance);

        Ok(Self {
            _instance: instance,
            device: logical_device,
            queue_handles,
            physical_device_surface,
        })
    }
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None) }
    }
}

impl Deref for LogicalDevice {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

pub struct QueueHandles {
    pub graphics: Queue,
    pub present: Queue,
}
