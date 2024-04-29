use std::{ffi::CString, ops::Deref};

use anyhow::Result;
use ash::{
    ext::debug_utils,
    vk::{make_api_version, ApplicationInfo, InstanceCreateInfo, API_VERSION_1_3},
    Entry,
};
use tracing::debug;

use crate::get_debug_messenger_create_info;

const API_VERSION: u32 = API_VERSION_1_3;

#[cfg(feature = "enable_validations")]
const ENABLE_VALIDATIONS: bool = true;
#[cfg(not(feature = "enable_validations"))]
const ENABLE_VALIDATIONS: bool = false;

pub struct Instance {
    instance: ash::Instance,
    entry: Entry,
}

impl Instance {
    /// Creates an Instance to interact with the core of Vulkan. Registers the needed extensions and
    /// layers, as well as basic information about the application.
    pub fn new(entry: Entry, required_extensions: Vec<&str>) -> Result<Self> {
        let appname = CString::new(env!("CARGO_PKG_NAME"))?;
        let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>()?;
        let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u32>()?;
        let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u32>()?;
        let app_version = make_api_version(0, version_major, version_minor, version_patch);

        let app_info = ApplicationInfo::default()
            .application_name(&appname)
            .application_version(app_version)
            .api_version(API_VERSION)
            .engine_name(&appname)
            .engine_version(app_version);

        let enabled_extension_names = Self::get_required_instance_extensions(required_extensions)?
            .into_iter()
            .map(|extension_name| CString::new(extension_name))
            .collect::<Result<Vec<_>, _>>()?;
        let enabled_extension_name_ptrs = enabled_extension_names
            .iter()
            .map(|extension_name| extension_name.as_ptr())
            .collect::<Vec<_>>();

        let enabled_layer_names = Self::gen_required_layers()
            .into_iter()
            .map(|layer_name| CString::new(layer_name))
            .collect::<Result<Vec<_>, _>>()?;
        let enabled_layer_name_pts = enabled_layer_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect::<Vec<_>>();

        let mut debug_messenger_create_info = get_debug_messenger_create_info();

        let instance_create_info = InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extension_name_ptrs)
            .enabled_layer_names(&enabled_layer_name_pts)
            .push_next(&mut debug_messenger_create_info);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        Ok(Self { instance, entry })
    }

    pub fn get_entry(&self) -> &Entry {
        &self.entry
    }

    /// Returns the needed instance exensions for Vulkan to function correctly.
    /// These always require the extensions necessary to interact with the native
    /// windowing system, and may include optional validation extensions if validations
    /// are enabled.
    fn get_required_instance_extensions(required_extensions: Vec<&str>) -> Result<Vec<&str>> {
        let mut enabled_extension_names = required_extensions.clone();
        if ENABLE_VALIDATIONS {
            enabled_extension_names.push(debug_utils::NAME.to_str()?);
        }
        Ok(enabled_extension_names)
    }

    /// Returns the required layers needed for Vulkan. Notably, includes the validation
    /// layer if validations are enabled.
    fn gen_required_layers() -> Vec<String> {
        let mut layer_names = vec![];
        if ENABLE_VALIDATIONS {
            layer_names = vec!["VK_LAYER_KHRONOS_validation".to_owned()];
        }
        debug!("Layers to enable: {}", layer_names.join(", "));
        return layer_names;
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) }
    }
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}
