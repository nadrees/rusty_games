use std::ptr;

use anyhow::Result;
use ash::{extensions::khr::Surface, vk::SurfaceKHR, Entry, Instance};
use glfw::PWindow;

pub struct SurfaceGuard {
    surface_loader: Surface,
    surface: SurfaceKHR,
}

impl SurfaceGuard {
    pub fn try_new(entry: &Entry, instance: &Instance, window: &PWindow) -> Result<Self> {
        let mut surface: std::mem::MaybeUninit<SurfaceKHR> = std::mem::MaybeUninit::uninit();
        window
            .create_window_surface(instance.handle(), ptr::null(), surface.as_mut_ptr())
            .result()?;
        let surface = unsafe { surface.assume_init() };
        let surface_loader = Surface::new(entry, instance);
        Ok(Self {
            surface,
            surface_loader,
        })
    }
}

impl Drop for SurfaceGuard {
    fn drop(&mut self) {
        unsafe { self.surface_loader.destroy_surface(self.surface, None) };
    }
}
