use std::{ops::Deref, rc::Rc};

use anyhow::Result;
use ash::vk::{Fence, FenceCreateFlags, FenceCreateInfo};

use super::logical_device::LogicalDeviceGuard;

pub struct FenceGuard {
    fence: Fence,
    logical_device: Rc<LogicalDeviceGuard>,
}

impl FenceGuard {
    pub fn try_new(logical_device: &Rc<LogicalDeviceGuard>, start_signaled: bool) -> Result<Self> {
        let mut fence_create_info = FenceCreateInfo::builder();
        if start_signaled {
            fence_create_info = fence_create_info.flags(FenceCreateFlags::SIGNALED);
        }
        let fence = unsafe { logical_device.create_fence(&fence_create_info, None) }?;
        Ok(Self {
            fence,
            logical_device: Rc::clone(logical_device),
        })
    }
}

impl Drop for FenceGuard {
    fn drop(&mut self) {
        unsafe { self.logical_device.destroy_fence(self.fence, None) }
    }
}

impl Deref for FenceGuard {
    type Target = Fence;

    fn deref(&self) -> &Self::Target {
        &self.fence
    }
}
