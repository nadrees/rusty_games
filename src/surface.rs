use std::{ops::Deref, rc::Rc};

use anyhow::Result;
use ash::{
    khr::surface,
    vk::{PhysicalDevice, PresentModeKHR, SurfaceCapabilitiesKHR, SurfaceFormatKHR, SurfaceKHR},
};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::Instance;

pub struct Surface {
    surface_fn: surface::Instance,
    surface_ptr: SurfaceKHR,
    // references to make sure we are dropped before these
    _instance: Rc<Instance>,
}

impl Surface {
    pub fn new(instance: &Rc<Instance>, window: &Window) -> Result<Self> {
        let surface_fn = surface::Instance::new(instance.get_entry(), instance);
        let surface_ptr = unsafe {
            ash_window::create_surface(
                instance.get_entry(),
                instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };
        Ok(Self {
            surface_fn,
            surface_ptr,
            _instance: instance.clone(),
        })
    }

    pub(crate) fn get_physical_device_surface_capabilities(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Result<SurfaceCapabilitiesKHR> {
        let capabilities = unsafe {
            self.surface_fn
                .get_physical_device_surface_capabilities(*physical_device, self.surface_ptr)
        }?;
        Ok(capabilities)
    }

    pub(crate) fn get_physical_device_surface_formats(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Result<Vec<SurfaceFormatKHR>> {
        let formats = unsafe {
            self.surface_fn
                .get_physical_device_surface_formats(*physical_device, self.surface_ptr)
        }?;
        Ok(formats)
    }

    pub(crate) fn get_physical_device_surface_present_modes(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Result<Vec<PresentModeKHR>> {
        let modes = unsafe {
            self.surface_fn
                .get_physical_device_surface_present_modes(*physical_device, self.surface_ptr)
        }?;
        Ok(modes)
    }

    pub(crate) fn get_physical_device_surface_support(
        &self,
        physical_device: &PhysicalDevice,
        queue_family_index: u32,
    ) -> Result<bool> {
        let surface_supports_device = unsafe {
            self.surface_fn.get_physical_device_surface_support(
                *physical_device,
                queue_family_index,
                self.surface_ptr,
            )
        }?;
        Ok(surface_supports_device)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.surface_fn.destroy_surface(self.surface_ptr, None) }
    }
}

impl Deref for Surface {
    type Target = SurfaceKHR;

    fn deref(&self) -> &Self::Target {
        &self.surface_ptr
    }
}
