use std::rc::Rc;

use anyhow::Result;
use ash::{
    extensions::{ext::DebugUtils, khr::Swapchain},
    vk::DebugUtilsMessengerCreateInfoEXT,
    Entry,
};
use glfw::{Glfw, PWindow};
use tracing::debug;

use crate::{
    get_debug_utils_create_info, physical_device::get_physical_device, DebugUtilsExtension,
    InstanceGuard, LogicalDeviceGuard, SurfaceGuard,
};

pub fn create_logical_device(
    entry: &Entry,
    glfw: &Glfw,
    window: &PWindow,
) -> Result<Rc<LogicalDeviceGuard>> {
    let extension_names = get_instance_extension_names(glfw)?;
    let layer_names = get_validation_layers();
    let instance_guard = if cfg!(debug_assertions) {
        let mut debug_create_info = get_debug_utils_create_info();
        InstanceGuard::try_new(
            entry,
            extension_names,
            layer_names,
            Some(&mut debug_create_info),
        )?
    } else {
        InstanceGuard::try_new::<DebugUtilsMessengerCreateInfoEXT>(
            &entry,
            extension_names,
            layer_names,
            None,
        )?
    };

    let surface = SurfaceGuard::try_new(&entry, &instance_guard, &window)?;

    let _debug_utils: Option<DebugUtilsExtension> = if cfg!(debug_assertions) {
        Some(DebugUtilsExtension::try_new(&entry, &instance_guard)?)
    } else {
        None
    };

    let physical_device =
        get_physical_device(&instance_guard, &surface, &get_device_extension_names()?)?;
    let logical_device = LogicalDeviceGuard::try_new(
        &instance_guard,
        physical_device,
        surface,
        &get_device_extension_names()?,
    )?;
    Ok(logical_device)
}

fn get_instance_extension_names(glfw: &Glfw) -> Result<Vec<String>> {
    let mut extension_names: Vec<String> = vec![DebugUtils::name().to_str()?.to_owned()];
    if let Some(mut glfw_required_extensions) = glfw.get_required_instance_extensions() {
        extension_names.append(&mut glfw_required_extensions);
    }
    debug!("Instance Extension names: {:?}", extension_names);
    Ok(extension_names)
}

fn get_device_extension_names() -> Result<Vec<String>> {
    let device_extension_names = vec![Swapchain::name().to_str()?.to_owned()];
    debug!("Device Extension names: {:?}", device_extension_names);
    Ok(device_extension_names)
}

fn get_validation_layers() -> Vec<&'static str> {
    let requested_validation_layers = {
        #[cfg(debug_assertions)]
        {
            vec!["VK_LAYER_KHRONOS_validation"]
        }
        #[cfg(not(debug_assertions))]
        {
            vec![]
        }
    };
    debug!("Validation layers: {:?}", requested_validation_layers);
    requested_validation_layers
}
