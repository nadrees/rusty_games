use anyhow::Result;
use ash::{
    vk::{ApplicationInfo, InstanceCreateInfo, API_VERSION_1_3},
    Entry, Instance,
};

const API_VERSION: u32 = API_VERSION_1_3;

/// Simple warpper around Instance to ensure expected Vulkan calls are made, especially cleanup on drop
pub struct VkInstanceGuard {
    instance: Instance,
}

impl VkInstanceGuard {
    pub fn try_new(entry: &Entry) -> Result<Self> {
        let app_info = ApplicationInfo {
            api_version: API_VERSION,
            ..Default::default()
        };
        let create_info = InstanceCreateInfo {
            p_application_info: &app_info,
            ..Default::default()
        };
        let instance = unsafe { entry.create_instance(&create_info, None)? };
        Ok(Self { instance })
    }

    pub fn get_instance<'a>(&'a self) -> &'a Instance {
        &self.instance
    }
}

impl Drop for VkInstanceGuard {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) }
    }
}
