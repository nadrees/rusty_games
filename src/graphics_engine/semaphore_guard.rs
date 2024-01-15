use std::{ops::Deref, rc::Rc};

use anyhow::Result;
use ash::vk::{Semaphore, SemaphoreCreateInfo};

use super::logical_device::LogicalDeviceGuard;

pub struct SemaphoreGuard {
    semaphore: Semaphore,
    logical_device: Rc<LogicalDeviceGuard>,
}

impl SemaphoreGuard {
    pub fn try_new(logical_device: &Rc<LogicalDeviceGuard>) -> Result<Self> {
        let create_info = SemaphoreCreateInfo::builder();
        let semaphore = unsafe { logical_device.create_semaphore(&create_info, None) }?;
        Ok(Self {
            semaphore,
            logical_device: Rc::clone(logical_device),
        })
    }
}

impl Drop for SemaphoreGuard {
    fn drop(&mut self) {
        unsafe { self.logical_device.destroy_semaphore(self.semaphore, None) }
    }
}

impl Deref for SemaphoreGuard {
    type Target = Semaphore;

    fn deref(&self) -> &Self::Target {
        &self.semaphore
    }
}
