use std::{fs, ops::Deref, rc::Rc};

use crate::LogicalDeviceGuard;
use anyhow::Result;
use ash::vk::{ShaderModule, ShaderModuleCreateInfo};

pub struct ShaderModuleGuard {
    shader_module: ShaderModule,
    logical_device: Rc<LogicalDeviceGuard>,
}

impl ShaderModuleGuard {
    pub fn try_new(file_name: &str, logical_device: &Rc<LogicalDeviceGuard>) -> Result<Self> {
        let bytes = fs::read(file_name)?;
        let (_, bytes, _) = unsafe { bytes.align_to::<u32>() };
        let create_info = ShaderModuleCreateInfo::builder().code(bytes).build();
        let shader_module = unsafe { logical_device.create_shader_module(&create_info, None) }?;
        Ok(Self {
            shader_module,
            logical_device: Rc::clone(logical_device),
        })
    }
}

impl Drop for ShaderModuleGuard {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_shader_module(self.shader_module, None)
        }
    }
}

impl Deref for ShaderModuleGuard {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.shader_module
    }
}
