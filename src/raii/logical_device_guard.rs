use std::{collections::HashSet, ffi::CString, ops::Deref, rc::Rc};

use anyhow::{anyhow, Result};
use ash::{
    vk::{DeviceCreateInfo, DeviceQueueCreateInfo, PhysicalDevice, Queue},
    Device,
};
use tracing::debug;

use crate::{queue_families::find_queue_families, InstanceGuard, SurfaceGuard};

/// RAII for logical device
pub struct LogicalDeviceGuard {
    device: Device,
    pub graphics_queue_family_index: u32,
    pub present_queue_family_index: u32,
    pub queue_indicies: Vec<u32>,
    // need to keep a reference to the instance to ensure we get
    // dropped before it does
    _instance: Rc<InstanceGuard>,
}

impl LogicalDeviceGuard {
    pub fn try_new(
        instance: &Rc<InstanceGuard>,
        physical_device: &PhysicalDevice,
        surface: &SurfaceGuard,
        device_extension_names: &Vec<String>,
    ) -> Result<Self> {
        let queue_family_indicies = find_queue_families(instance, &physical_device, surface)?;
        debug!("Queue family indicies: {:?}", queue_family_indicies);

        let graphics_queue_family_index = queue_family_indicies
            .graphics_family
            .ok_or(anyhow!("No graphics family index"))?;
        let present_queue_family_index = queue_family_indicies
            .present_family
            .ok_or(anyhow!("No present family index"))?;

        let queue_priorities = vec![1.0f32];
        let queue_indicies = Vec::from_iter(HashSet::from([
            graphics_queue_family_index,
            present_queue_family_index,
        ]));

        let device_queue_create_infos = queue_indicies
            .iter()
            .map(|queue_family_index| {
                DeviceQueueCreateInfo::builder()
                    .queue_family_index(*queue_family_index)
                    .queue_priorities(&queue_priorities)
                    .build()
            })
            .collect::<Vec<_>>();

        let device_extension_names: Vec<CString> = device_extension_names
            .into_iter()
            .map(|extension_name| CString::new(extension_name.to_owned()))
            .collect::<Result<_, _>>()?;
        let device_extension_name_ptrs = device_extension_names
            .iter()
            .map(|device_extension| device_extension.as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = DeviceCreateInfo::builder()
            .queue_create_infos(&device_queue_create_infos)
            .enabled_extension_names(&device_extension_name_ptrs);
        let logical_device =
            unsafe { instance.create_device(*physical_device, &device_create_info, None)? };

        Ok(Self {
            device: logical_device,
            graphics_queue_family_index,
            present_queue_family_index,
            queue_indicies,
            _instance: Rc::clone(instance),
        })
    }

    pub fn get_graphics_queue(&self) -> Queue {
        unsafe {
            self.device
                .get_device_queue(self.graphics_queue_family_index, 0)
        }
    }

    pub fn get_present_queue(&self) -> Queue {
        unsafe {
            self.device
                .get_device_queue(self.present_queue_family_index, 0)
        }
    }
}

impl Deref for LogicalDeviceGuard {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Drop for LogicalDeviceGuard {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None) }
    }
}
