use anyhow::Result;
use ash::vk::{PhysicalDevice, QueueFamilyProperties, QueueFlags};

use super::{instance_guard::InstanceGuard, surface_guard::SurfaceGuard};

#[derive(Debug)]
pub struct QueueFamilyIndicies {
    /// family capable of runing graphics related commands
    pub graphics_family: Option<u32>,
    /// family capable of displaying results on the screen
    pub present_family: Option<u32>,
}

pub fn find_queue_families(
    instance: &InstanceGuard,
    device: &PhysicalDevice,
    surface: &SurfaceGuard,
) -> Result<QueueFamilyIndicies> {
    fn find_queue_family_index(
        queue_family_properties: &Vec<QueueFamilyProperties>,
        flags: QueueFlags,
    ) -> Option<u32> {
        queue_family_properties
            .into_iter()
            .enumerate()
            .filter_map(|(index, queue_family_props)| {
                if queue_family_props.queue_flags.contains(flags) {
                    return Some(index as u32);
                } else {
                    return None;
                }
            })
            .collect::<Vec<_>>()
            .first()
            .cloned()
    }

    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(*device) };
    let graphics_family = find_queue_family_index(&queue_family_properties, QueueFlags::GRAPHICS);

    let mut present_family = None;
    for index in 0..queue_family_properties.len() as u32 {
        let supports_present = surface.get_physical_device_surface_support(device, index)?;
        if supports_present {
            present_family = Some(index);
            break;
        }
    }

    Ok(QueueFamilyIndicies {
        graphics_family,
        present_family,
    })
}
