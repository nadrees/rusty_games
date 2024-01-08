use std::{collections::HashSet, ffi::CStr};

use anyhow::{anyhow, Result};
use ash::vk::PhysicalDevice;

use super::{
    instance_guard::InstanceGuard, query_swap_chain_support, queue_families::find_queue_families,
    surface_guard::SurfaceGuard,
};

pub fn get_physical_device(
    instance: &InstanceGuard,
    surface: &SurfaceGuard,
    extension_names: &Vec<String>,
) -> Result<PhysicalDevice> {
    fn device_supports_required_queues(
        instance: &InstanceGuard,
        surface: &SurfaceGuard,
        physical_device: &PhysicalDevice,
    ) -> Result<bool> {
        let queue_families = find_queue_families(instance, &physical_device, surface)?;
        Ok(queue_families.graphics_family.is_some() && queue_families.present_family.is_some())
    }

    fn device_supports_required_extensions(
        instance: &InstanceGuard,
        physical_device: &PhysicalDevice,
        extension_names: &Vec<String>,
    ) -> Result<bool> {
        let extensions =
            unsafe { instance.enumerate_device_extension_properties(*physical_device) }?;
        let mut available_extension_names = HashSet::new();
        for extension in extensions {
            let extension_name = unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) }
                .to_str()?
                .to_owned();
            available_extension_names.insert(extension_name);
        }

        Ok(extension_names
            .iter()
            .all(|extension_name| available_extension_names.contains(extension_name)))
    }

    fn device_supports_swapchain(surface: &SurfaceGuard, device: &PhysicalDevice) -> Result<bool> {
        let swapchain_support_details = query_swap_chain_support(surface, device)?;
        Ok(!swapchain_support_details.formats.is_empty()
            && !swapchain_support_details.present_modes.is_empty())
    }

    let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
    for physical_device in physical_devices {
        if device_supports_required_queues(instance, surface, &physical_device)?
            && device_supports_required_extensions(instance, &physical_device, extension_names)?
            && device_supports_swapchain(surface, &physical_device)?
        {
            return Ok(physical_device);
        }
    }
    Err(anyhow!("no suitable graphics cards found!"))
}
