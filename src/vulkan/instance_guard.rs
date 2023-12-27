use std::ffi::CString;

use anyhow::Result;
use ash::{
    vk::{make_api_version, ApplicationInfo, InstanceCreateInfo, API_VERSION_1_3},
    Entry, Instance,
};
use glfw::PWindow;

use crate::vulkan::extensions_registry::{self, DebugUtilsGuard};
use crate::vulkan::layers_registry;

use super::{
    logical_device::LogicalDevice, physical_device_manager::PhysicalDevice,
    surface_guard::SurfaceGuard,
};

const API_VERSION: u32 = API_VERSION_1_3;

/// Simple warpper around Instance to ensure expected Vulkan calls are made, especially cleanup on drop
pub struct VkInstanceGuard {
    pub instance: Instance,
    logical_devices: Vec<LogicalDevice>,
    surface: Option<SurfaceGuard>,
}

impl VkInstanceGuard {
    pub fn try_new(
        entry: &Entry,
        window: &PWindow,
        additional_extensions: Option<Vec<String>>,
    ) -> Result<Self> {
        let appname = CString::new(env!("CARGO_PKG_NAME")).unwrap();

        let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap();
        let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap();
        let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap();
        let app_version = make_api_version(0, version_major, version_minor, version_patch);

        let app_info = ApplicationInfo::builder()
            .application_name(&appname)
            .application_version(app_version)
            .api_version(API_VERSION)
            .engine_name(&appname)
            .engine_version(app_version);

        let layers = layers_registry::get_names()
            .into_iter()
            .map(|layer| CString::new(layer))
            .collect::<Result<Vec<_>, _>>()?;
        let layer_names_pointers: Vec<*const i8> = layers
            .iter()
            .map(|layer| layer.as_ptr())
            .collect::<Vec<_>>();

        let mut extensions: Vec<String> = extensions_registry::get_names();
        if let Some(additional_extensions) = additional_extensions {
            extensions.append(additional_extensions.clone().as_mut());
        }
        let extensions = extensions
            .into_iter()
            .map(|extension| CString::new(extension))
            .collect::<Result<Vec<_>, _>>()?;
        let extensions_names_pointers: Vec<*const i8> = extensions
            .iter()
            .map(|extension| extension.as_ptr())
            .collect::<Vec<_>>();

        let mut create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_names_pointers)
            .enabled_extension_names(&extensions_names_pointers);

        let mut debug_create_info = DebugUtilsGuard::get_debug_create_info();
        if cfg!(debug_assertions) {
            create_info = create_info.push_next(&mut debug_create_info);
        }

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        extensions_registry::create_extensions(entry, &instance)?;

        let surface = SurfaceGuard::try_new(entry, &instance, window)?;

        Ok(Self {
            instance,
            logical_devices: vec![],
            surface: Some(surface),
        })
    }

    /// Creates a logical device for interfacing with the selected physical device. Will automatically track the logical device
    /// and destroy it when this instance is dropped.
    pub fn create_logical_device(
        &mut self,
        physical_device: &PhysicalDevice,
    ) -> Result<&LogicalDevice> {
        let logical_device = LogicalDevice::try_new(&self, physical_device)?;
        self.logical_devices.push(logical_device);
        Ok(&self.logical_devices[self.logical_devices.len() - 1])
    }
}

impl Drop for VkInstanceGuard {
    fn drop(&mut self) {
        // need to clear collections so that their destructors can run before the rest of this one does
        self.logical_devices.clear();
        // clear fields
        self.surface = None;
        // destroy self
        unsafe { self.instance.destroy_instance(None) }
    }
}
