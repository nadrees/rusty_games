use std::{mem::MaybeUninit, ops::Deref, ptr, rc::Rc};

use anyhow::Result;
use ash::{
    extensions::khr::Surface,
    vk::{PhysicalDevice, PresentModeKHR, SurfaceCapabilitiesKHR, SurfaceFormatKHR, SurfaceKHR},
    Entry,
};
use glfw::PWindow;

use crate::InstanceGuard;

/// RAII for Surface
pub struct SurfaceGuard {
    surface: Surface,
    surface_ptr: SurfaceKHR,
    // need to keep a reference to instance to ensure we get dropped before it
    _instance: Rc<InstanceGuard>,
}

impl SurfaceGuard {
    pub fn try_new(entry: &Entry, instance: &Rc<InstanceGuard>, window: &PWindow) -> Result<Self> {
        let mut surface_ptr: std::mem::MaybeUninit<SurfaceKHR> = MaybeUninit::uninit();
        window
            .create_window_surface(instance.handle(), ptr::null(), surface_ptr.as_mut_ptr())
            .result()?;
        let surface_ptr = unsafe { surface_ptr.assume_init() };
        let surface = Surface::new(&entry, &instance.instance);
        Ok(Self {
            surface,
            surface_ptr,
            _instance: Rc::clone(instance),
        })
    }

    pub fn get_capabilities(&self, device: &PhysicalDevice) -> Result<SurfaceCapabilitiesKHR> {
        Ok(unsafe {
            self.surface
                .get_physical_device_surface_capabilities(*device, self.surface_ptr)
        }?)
    }

    pub fn get_surface_formats(&self, device: &PhysicalDevice) -> Result<Vec<SurfaceFormatKHR>> {
        Ok(unsafe {
            self.surface
                .get_physical_device_surface_formats(*device, self.surface_ptr)
        }?)
    }

    pub fn get_presentation_modes(&self, device: &PhysicalDevice) -> Result<Vec<PresentModeKHR>> {
        Ok(unsafe {
            self.surface
                .get_physical_device_surface_present_modes(*device, self.surface_ptr)
        }?)
    }

    pub fn get_physical_device_surface_support(
        &self,
        device: &PhysicalDevice,
        index: u32,
    ) -> Result<bool> {
        Ok(unsafe {
            self.surface
                .get_physical_device_surface_support(*device, index, self.surface_ptr)
        }?)
    }
}

impl Drop for SurfaceGuard {
    fn drop(&mut self) {
        unsafe { self.surface.destroy_surface(self.surface_ptr, None) }
    }
}

impl Deref for SurfaceGuard {
    type Target = SurfaceKHR;

    fn deref(&self) -> &Self::Target {
        &self.surface_ptr
    }
}
