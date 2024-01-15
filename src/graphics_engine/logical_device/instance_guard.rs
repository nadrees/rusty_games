use std::{ffi::CString, ops::Deref, rc::Rc};

use anyhow::Result;
use ash::{
    vk::{
        make_api_version, ApplicationInfo, ExtendsInstanceCreateInfo, InstanceCreateInfo,
        API_VERSION_1_3,
    },
    Entry, Instance,
};
use tracing::debug;

const API_VERSION: u32 = API_VERSION_1_3;

/// RAII for Instance
pub struct InstanceGuard {
    pub instance: Instance,
}

impl InstanceGuard {
    pub fn try_new<'a, T>(
        entry: &Entry,
        extension_names: Vec<String>,
        layer_names: Vec<&str>,
        next: Option<&'a mut T>,
    ) -> Result<Rc<Self>>
    where
        T: ExtendsInstanceCreateInfo,
    {
        let appname = CString::new(env!("CARGO_PKG_NAME")).unwrap();
        let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap();
        let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap();
        let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap();
        let app_version = make_api_version(0, version_major, version_minor, version_patch);

        let application_info = ApplicationInfo::builder()
            .application_name(&appname)
            .application_version(app_version)
            .api_version(API_VERSION)
            .engine_name(&appname)
            .engine_version(app_version);

        let extension_names: Vec<CString> = extension_names
            .into_iter()
            .map(|extension_name| CString::new(extension_name))
            .collect::<Result<_, _>>()?;
        let extension_name_pointers = extension_names
            .iter()
            .map(|extension_name| extension_name.as_ptr())
            .collect::<Vec<_>>();

        let layer_names: Vec<CString> = layer_names
            .into_iter()
            .map(|layer_name| CString::new(layer_name))
            .collect::<Result<_, _>>()?;
        let layer_name_pointers = layer_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect::<Vec<_>>();

        let mut instance_create_info = InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_extension_names(&extension_name_pointers)
            .enabled_layer_names(&layer_name_pointers);
        if let Some(next) = next {
            instance_create_info = instance_create_info.push_next(next);
        }

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        Ok(Rc::new(Self { instance }))
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        debug!("Dropping InstanceGuard");
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

impl Deref for InstanceGuard {
    type Target = Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}
