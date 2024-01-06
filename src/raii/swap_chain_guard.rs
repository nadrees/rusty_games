use ash::vk::{PhysicalDevice, PresentModeKHR, SurfaceCapabilitiesKHR, SurfaceFormatKHR};

use crate::SurfaceGuard;
use anyhow::Result;

pub struct SwapChainSupportDetails {
    pub capabilities: SurfaceCapabilitiesKHR,
    pub formats: Vec<SurfaceFormatKHR>,
    pub present_modes: Vec<PresentModeKHR>,
}

pub fn query_swap_chain_support(
    surface: &SurfaceGuard,
    device: &PhysicalDevice,
) -> Result<SwapChainSupportDetails> {
    let capabilities = surface.get_capabilities(device)?;
    let formats = surface.get_surface_formats(device)?;
    let present_modes = surface.get_presentation_modes(device)?;
    Ok(SwapChainSupportDetails {
        capabilities,
        formats,
        present_modes,
    })
}
