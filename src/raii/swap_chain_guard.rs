use std::{ffi::c_uint, rc::Rc};

use ash::{
    extensions::khr::Swapchain,
    vk::{
        ColorSpaceKHR, CompositeAlphaFlagsKHR, Extent2D, Format, ImageUsageFlags, PhysicalDevice,
        PresentModeKHR, SharingMode, SurfaceCapabilitiesKHR, SurfaceFormatKHR,
        SwapchainCreateInfoKHR, SwapchainKHR,
    },
    Entry,
};
use glfw::PWindow;
use tracing::debug;

use crate::{LogicalDeviceGuard, SurfaceGuard};
use anyhow::Result;

use super::image_view_guard::ImageViewGuard;

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

pub struct SwapChainGuard {
    handle: SwapchainKHR,
    pub extent: Extent2D,
    image_views: Vec<ImageViewGuard>,
    pub surface_format: SurfaceFormatKHR,
    swapchain: Swapchain,
}

impl SwapChainGuard {
    pub fn try_new(
        entry: &Entry,
        logical_device: &Rc<LogicalDeviceGuard>,
        window: &PWindow,
    ) -> Result<Self> {
        let swap_chain_support_details =
            query_swap_chain_support(&logical_device.surface, &logical_device.physical_device)?;
        let surface_format = choose_swap_chain_format(&swap_chain_support_details.formats);
        let presentation_mode = choose_presentation_mode(&swap_chain_support_details.present_modes);
        let extent = choose_swap_extent(&window, &swap_chain_support_details.capabilities)?;
        let mut image_count = swap_chain_support_details.capabilities.min_image_count + 1;
        if swap_chain_support_details.capabilities.max_image_count > 0 {
            image_count = image_count.clamp(
                swap_chain_support_details.capabilities.min_image_count,
                swap_chain_support_details.capabilities.max_image_count,
            );
        }
        let graphics_and_present_queues_are_same =
            logical_device.graphics_queue_family_index == logical_device.present_queue_family_index;
        let swap_chain_creation_info = SwapchainCreateInfoKHR::builder()
            // ignore alpha channel
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            // enable clipping to discard pixels that are hidden by something else (like another window)
            .clipped(true)
            // not doing sterioscopic processing, only need 1 layer
            .image_array_layers(1)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_format(surface_format.format)
            // if queues are the same, use exclusive mode for best perfomance
            .image_sharing_mode(match graphics_and_present_queues_are_same {
                true => SharingMode::EXCLUSIVE,
                false => SharingMode::CONCURRENT,
            })
            // we're rendering images, so set usage as a color attachment
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .min_image_count(image_count)
            // no extra transforms - just pass in current transform
            .pre_transform(swap_chain_support_details.capabilities.current_transform)
            .present_mode(presentation_mode)
            .queue_family_indices(match graphics_and_present_queues_are_same {
                true => &[],
                false => &logical_device.queue_indicies,
            })
            .surface(*logical_device.surface);
        let swap_chain =
            Swapchain::new_from_instance(&entry, &logical_device.instance, logical_device.handle());
        let swap_chain_handle =
            unsafe { swap_chain.create_swapchain(&swap_chain_creation_info, None) }?;

        let images = unsafe { swap_chain.get_swapchain_images(swap_chain_handle) }?;
        let image_views = images
            .into_iter()
            .map(|image| ImageViewGuard::try_new(image, &logical_device, surface_format))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            swapchain: swap_chain,
            handle: swap_chain_handle,
            image_views,
            extent,
            surface_format: surface_format.to_owned(),
        })
    }

    pub fn get_images(&self) -> Vec<&ImageViewGuard> {
        self.image_views.iter().collect()
    }
}

impl Drop for SwapChainGuard {
    fn drop(&mut self) {
        debug!("Dropping SwapChainGuard");
        unsafe { self.swapchain.destroy_swapchain(self.handle, None) }
    }
}

fn choose_swap_chain_format(available_formats: &Vec<SurfaceFormatKHR>) -> &SurfaceFormatKHR {
    available_formats
        .iter()
        .find(|format| {
            format.format == Format::B8G8R8A8_SRGB
                && format.color_space == ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| available_formats.first().expect("No availabe formats!"))
}

fn choose_presentation_mode(available_modes: &Vec<PresentModeKHR>) -> PresentModeKHR {
    if available_modes.contains(&PresentModeKHR::MAILBOX) {
        PresentModeKHR::MAILBOX
    } else {
        PresentModeKHR::FIFO
    }
}

fn choose_swap_extent(window: &PWindow, capabilities: &SurfaceCapabilitiesKHR) -> Result<Extent2D> {
    if capabilities.current_extent.width != c_uint::MAX {
        return Ok(capabilities.current_extent);
    }
    let (width, height) = window.get_framebuffer_size();
    let width = u32::try_from(width)?;
    let height = u32::try_from(height)?;
    Ok(Extent2D {
        width: width.clamp(
            capabilities.min_image_extent.width,
            capabilities.max_image_extent.width,
        ),
        height: height.clamp(
            capabilities.min_image_extent.height,
            capabilities.max_image_extent.height,
        ),
    })
}
