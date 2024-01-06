use std::ffi::c_uint;

use anyhow::{anyhow, Result};
use ash::{
    extensions::{ext::DebugUtils, khr::Swapchain},
    vk::{
        ColorSpaceKHR, CompositeAlphaFlagsKHR, DebugUtilsMessengerCreateInfoEXT, Extent2D, Format,
        ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, ImageViewCreateInfo,
        ImageViewType, PresentModeKHR, SharingMode, SurfaceCapabilitiesKHR, SurfaceFormatKHR,
        SwapchainCreateInfoKHR,
    },
    Entry,
};
use glfw::{fail_on_errors, Glfw, PWindow};
use rusty_games::{
    get_debug_utils_create_info, init_logging, physical_device::get_physical_device,
    query_swap_chain_support, DebugUtilsExtension, InstanceGuard, LogicalDeviceGuard, SurfaceGuard,
};
use tracing::debug;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_TITLE: &str = "Hello, Triangle";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let mut glfw = glfw::init(fail_on_errors!())?;
    glfw.window_hint(glfw::WindowHint::Visible(true));
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
    glfw.window_hint(glfw::WindowHint::Resizable(false));
    let (window, _) = glfw
        .create_window(
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            WINDOW_TITLE,
            glfw::WindowMode::Windowed,
        )
        .ok_or(anyhow!("Failed to create window"))?;

    let entry = Entry::linked();

    let extension_names = get_instance_extension_names(&glfw)?;
    let layer_names = get_validation_layers();
    let instance_guard = if cfg!(debug_assertions) {
        let mut debug_create_info = get_debug_utils_create_info();
        InstanceGuard::try_new(
            &entry,
            extension_names,
            layer_names,
            Some(&mut debug_create_info),
        )?
    } else {
        InstanceGuard::try_new::<DebugUtilsMessengerCreateInfoEXT>(
            &entry,
            extension_names,
            layer_names,
            None,
        )?
    };

    let surface = SurfaceGuard::try_new(&entry, &instance_guard, &window)?;

    let _debug_utils: Option<DebugUtilsExtension> = if cfg!(debug_assertions) {
        Some(DebugUtilsExtension::try_new(&entry, &instance_guard)?)
    } else {
        None
    };

    let physical_device =
        get_physical_device(&instance_guard, &surface, &get_device_extension_names()?)?;
    let logical_device = LogicalDeviceGuard::try_new(
        &instance_guard,
        &physical_device,
        &surface,
        &get_device_extension_names()?,
    )?;

    let graphics_queue = logical_device.get_graphics_queue();
    let present_queue = logical_device.get_present_queue();

    let swap_chain_support_details = query_swap_chain_support(&surface, &physical_device)?;
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
        .surface(*surface);
    let swap_chain =
        Swapchain::new_from_instance(&entry, &instance_guard.instance, logical_device.handle());
    let swap_chain_handle =
        unsafe { swap_chain.create_swapchain(&swap_chain_creation_info, None) }?;

    let swap_chain_images = unsafe { swap_chain.get_swapchain_images(swap_chain_handle) }?;
    let image_views = swap_chain_images
        .into_iter()
        .map(|image| {
            let image_view_create_info = ImageViewCreateInfo::builder()
                .image(image)
                .view_type(ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .subresource_range(
                    ImageSubresourceRange::builder()
                        .aspect_mask(ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                );
            unsafe { logical_device.create_image_view(&image_view_create_info, None) }
        })
        .collect::<Result<Vec<_>, _>>()?;

    while !window.should_close() {
        glfw.wait_events();
    }

    unsafe {
        image_views
            .into_iter()
            .for_each(|image_view| logical_device.destroy_image_view(image_view, None));
        swap_chain.destroy_swapchain(swap_chain_handle, None);
    };

    Ok(())
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

fn get_instance_extension_names(glfw: &Glfw) -> Result<Vec<String>> {
    let mut extension_names: Vec<String> = vec![DebugUtils::name().to_str()?.to_owned()];
    if let Some(mut glfw_required_extensions) = glfw.get_required_instance_extensions() {
        extension_names.append(&mut glfw_required_extensions);
    }
    debug!("Instance Extension names: {:?}", extension_names);
    Ok(extension_names)
}

fn get_device_extension_names() -> Result<Vec<String>> {
    let device_extension_names = vec![Swapchain::name().to_str()?.to_owned()];
    debug!("Device Extension names: {:?}", device_extension_names);
    Ok(device_extension_names)
}

fn get_validation_layers() -> Vec<&'static str> {
    let requested_validation_layers = {
        #[cfg(debug_assertions)]
        {
            vec!["VK_LAYER_KHRONOS_validation"]
        }
        #[cfg(not(debug_assertions))]
        {
            vec![]
        }
    };
    debug!("Validation layers: {:?}", requested_validation_layers);
    requested_validation_layers
}
