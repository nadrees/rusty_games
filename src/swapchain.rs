use std::{collections::HashSet, ops::Deref, rc::Rc};

use anyhow::Result;
use ash::{
    khr::swapchain,
    vk::{
        CompositeAlphaFlagsKHR, Extent2D, Fence, Image, ImageUsageFlags, Semaphore, SharingMode,
        SurfaceFormatKHR, SwapchainCreateInfoKHR, SwapchainKHR,
    },
};
use winit::window::Window;

use crate::{Instance, LogicalDevice};

pub struct Swapchain {
    swapchain_fn: swapchain::Device,
    swapchain_ptr: SwapchainKHR,
    extent: Extent2D,
    surface_format: SurfaceFormatKHR,
    // references we need to keep to ensure
    // we are cleaned up before they are
    _instance: Rc<Instance>,
    _logical_device: Rc<LogicalDevice>,
    _window: Rc<Window>,
}

impl Swapchain {
    pub fn new(
        instance: &Rc<Instance>,
        window: &Rc<Window>,
        logical_device: &Rc<LogicalDevice>,
    ) -> Result<Self> {
        let queue_indicies = logical_device.get_queue_family_indicies();
        let queue_family_indicies = Vec::from_iter(HashSet::from([
            queue_indicies.graphics_family.unwrap() as u32,
            queue_indicies.present_family.unwrap() as u32,
        ]));

        let swap_chain_support = logical_device.get_swapchain_support_details();
        let surface_format = swap_chain_support.choose_swap_surface_format();
        let present_mode = swap_chain_support.choose_swap_present_mode();
        let extent = swap_chain_support.choose_swap_extent(window);
        let image_count = swap_chain_support.get_image_count();

        let mut swap_chain_creation_info = SwapchainCreateInfoKHR::default()
            .surface(***logical_device.get_surface())
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .present_mode(present_mode)
            // always 1 unless doing sterioscopic 3D
            .image_array_layers(1)
            // use images as color attachments for drawing color pictures to
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            // no transform
            .pre_transform(swap_chain_support.capabilities.current_transform)
            // ignore alpha channel
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            // enable clipping, to discard pixels that aren't visible
            .clipped(true)
            .old_swapchain(SwapchainKHR::null());
        if queue_family_indicies.len() == 1 {
            swap_chain_creation_info =
                swap_chain_creation_info.image_sharing_mode(SharingMode::EXCLUSIVE);
        } else {
            swap_chain_creation_info = swap_chain_creation_info
                .image_sharing_mode(SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indicies);
        }

        let swapchain_device = swapchain::Device::new(instance, &logical_device);
        let swapchain =
            unsafe { swapchain_device.create_swapchain(&swap_chain_creation_info, None) }?;

        let extent = logical_device
            .get_swapchain_support_details()
            .choose_swap_extent(window);
        let surface_format = logical_device
            .get_swapchain_support_details()
            .choose_swap_surface_format();

        Ok(Self {
            _instance: Rc::clone(instance),
            swapchain_fn: swapchain_device,
            swapchain_ptr: swapchain,
            _logical_device: Rc::clone(logical_device),
            extent,
            surface_format: *surface_format,
            _window: Rc::clone(window),
        })
    }

    pub fn get_swapchain_images(&self) -> Result<Vec<Image>> {
        let images = unsafe { self.swapchain_fn.get_swapchain_images(self.swapchain_ptr)? };
        Ok(images)
    }

    pub fn acquire_next_image_index(&self, signal_semaphore: &Semaphore) -> Result<u32> {
        let (index, _) = unsafe {
            self.swapchain_fn.acquire_next_image(
                self.swapchain_ptr,
                u64::MAX,
                *signal_semaphore,
                Fence::null(),
            )?
        };
        Ok(index)
    }

    pub fn get_handle(&self) -> &SwapchainKHR {
        &self.swapchain_ptr
    }

    pub fn get_extent(&self) -> &Extent2D {
        &self.extent
    }

    pub fn get_surface_format(&self) -> &SurfaceFormatKHR {
        &self.surface_format
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_fn
                .destroy_swapchain(self.swapchain_ptr, None)
        }
    }
}

impl Deref for Swapchain {
    type Target = swapchain::Device;

    fn deref(&self) -> &Self::Target {
        &self.swapchain_fn
    }
}
