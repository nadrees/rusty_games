use std::{
    collections::HashSet,
    ffi::{c_uint, CStr, CString},
    mem::MaybeUninit,
    ptr,
};

use anyhow::{anyhow, Result};
use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{Surface, Swapchain},
    },
    vk::{
        make_api_version, ApplicationInfo, Bool32, ColorSpaceKHR, CompositeAlphaFlagsKHR,
        DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
        DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT,
        DebugUtilsMessengerCreateInfoEXTBuilder, DebugUtilsMessengerEXT, DeviceCreateInfo,
        DeviceQueueCreateInfo, Extent2D, Format, ImageAspectFlags, ImageSubresourceRange,
        ImageUsageFlags, ImageViewCreateInfo, ImageViewType, InstanceCreateInfo, PhysicalDevice,
        PresentModeKHR, QueueFamilyProperties, QueueFlags, SharingMode, SurfaceCapabilitiesKHR,
        SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR, API_VERSION_1_3,
    },
    Entry, Instance,
};
use glfw::{fail_on_errors, Glfw, PWindow};
use rusty_games::{init_logging, QueueFamilyIndicies, SwapChainSupportDetails};
use tracing::{debug, event, Level};

const API_VERSION: u32 = API_VERSION_1_3;
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

    let appname = CString::new(env!("CARGO_PKG_NAME")).unwrap();
    let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap();
    let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap();
    let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap();
    let app_version = make_api_version(0, version_major, version_minor, version_patch);

    let application_info = ApplicationInfo::builder()
        .application_name(&appname)
        .application_version(app_version)
        .api_version(API_VERSION)
        .engine_name(&appname)
        .engine_version(app_version);

    let extension_names: Vec<CString> = get_instance_extension_names(&glfw)?
        .into_iter()
        .map(|extension_name| CString::new(extension_name))
        .collect::<Result<_, _>>()?;
    let extension_name_pointers = extension_names
        .iter()
        .map(|extension_name| extension_name.as_ptr())
        .collect::<Vec<_>>();

    let layer_names: Vec<CString> = get_validation_layers()
        .into_iter()
        .map(|layer_name| CString::new(layer_name))
        .collect::<Result<_, _>>()?;
    let layer_name_pointers = layer_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect::<Vec<_>>();

    let mut instance_create_info = InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_extension_names(&extension_name_pointers)
        .enabled_layer_names(&layer_name_pointers);

    let mut debug_create_info = get_debug_utils_create_info();
    if cfg!(debug_assertions) {
        instance_create_info = instance_create_info.push_next(&mut debug_create_info);
    }

    let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

    let mut surface_ptr: std::mem::MaybeUninit<SurfaceKHR> = MaybeUninit::uninit();
    window
        .create_window_surface(instance.handle(), ptr::null(), surface_ptr.as_mut_ptr())
        .result()?;
    let surface_ptr = unsafe { surface_ptr.assume_init() };
    let surface = Surface::new(&entry, &instance);

    let debug_utils = create_debug_utils(&entry, &instance)?;

    let physical_device = get_physical_device(&instance, &surface, &surface_ptr)?;
    let queue_family_indicies =
        find_queue_families(&instance, &physical_device, &surface, &surface_ptr)?;
    debug!("Queue family indicies: {:?}", queue_family_indicies);

    let graphics_queue_family_index = queue_family_indicies
        .graphics_family
        .ok_or(anyhow!("No graphics family index"))?;
    let present_queue_family_index = queue_family_indicies
        .present_family
        .ok_or(anyhow!("No present family index"))?;

    let queue_priorities = vec![1.0f32];
    let queue_indexes = Vec::from_iter(HashSet::from([
        graphics_queue_family_index,
        present_queue_family_index,
    ]));
    let device_queue_create_infos = queue_indexes
        .iter()
        .map(|queue_family_index| {
            DeviceQueueCreateInfo::builder()
                .queue_family_index(*queue_family_index)
                .queue_priorities(&queue_priorities)
                .build()
        })
        .collect::<Vec<_>>();

    let device_extension_names: Vec<CString> = get_device_extension_names()?
        .into_iter()
        .map(|extension_name| CString::new(extension_name))
        .collect::<Result<_, _>>()?;
    let device_extension_name_ptrs = device_extension_names
        .iter()
        .map(|device_extension| device_extension.as_ptr())
        .collect::<Vec<_>>();

    let device_create_info = DeviceCreateInfo::builder()
        .queue_create_infos(&device_queue_create_infos)
        .enabled_extension_names(&device_extension_name_ptrs);
    let logical_device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };

    let graphics_queue = unsafe { logical_device.get_device_queue(graphics_queue_family_index, 0) };
    let present_queue = unsafe { logical_device.get_device_queue(present_queue_family_index, 0) };

    let swap_chain_support_details =
        query_swap_chain_support(&surface, &surface_ptr, &physical_device)?;
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
        graphics_queue_family_index == present_queue_family_index;
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
            false => &queue_indexes,
        })
        .surface(surface_ptr);
    let swap_chain = Swapchain::new_from_instance(&entry, &instance, logical_device.handle());
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

    if let Some((debug_utils, debug_utils_extension)) = debug_utils {
        unsafe { debug_utils.destroy_debug_utils_messenger(debug_utils_extension, None) }
    }
    unsafe {
        image_views
            .into_iter()
            .for_each(|image_view| logical_device.destroy_image_view(image_view, None));
        swap_chain.destroy_swapchain(swap_chain_handle, None);
        logical_device.destroy_device(None);
        surface.destroy_surface(surface_ptr, None);
        instance.destroy_instance(None);
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

fn query_swap_chain_support(
    surface: &Surface,
    surface_ptr: &SurfaceKHR,
    device: &PhysicalDevice,
) -> Result<SwapChainSupportDetails> {
    let capabilities =
        unsafe { surface.get_physical_device_surface_capabilities(*device, *surface_ptr) }?;
    let formats = unsafe { surface.get_physical_device_surface_formats(*device, *surface_ptr) }?;
    let present_modes =
        unsafe { surface.get_physical_device_surface_present_modes(*device, *surface_ptr) }?;
    Ok(SwapChainSupportDetails {
        capabilities,
        formats,
        present_modes,
    })
}

fn get_physical_device(
    instance: &Instance,
    surface: &Surface,
    surface_ptr: &SurfaceKHR,
) -> Result<PhysicalDevice> {
    fn device_supports_required_queues(
        instance: &Instance,
        surface: &Surface,
        surface_ptr: &SurfaceKHR,
        physical_device: &PhysicalDevice,
    ) -> Result<bool> {
        let queue_families = find_queue_families(instance, &physical_device, surface, surface_ptr)?;
        Ok(queue_families.graphics_family.is_some() && queue_families.present_family.is_some())
    }

    fn device_supports_required_extensions(
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<bool> {
        let required_extension_names = get_device_extension_names()?;

        let extensions =
            unsafe { instance.enumerate_device_extension_properties(*physical_device) }?;
        let mut available_extension_names = HashSet::new();
        for extension in extensions {
            let extension_name = unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) }
                .to_str()?
                .to_owned();
            available_extension_names.insert(extension_name);
        }

        Ok(required_extension_names
            .iter()
            .all(|extension_name| available_extension_names.contains(extension_name)))
    }

    fn device_supports_swapchain(
        surface: &Surface,
        surface_ptr: &SurfaceKHR,
        device: &PhysicalDevice,
    ) -> Result<bool> {
        let swapchain_support_details = query_swap_chain_support(surface, surface_ptr, device)?;
        Ok(!swapchain_support_details.formats.is_empty()
            && !swapchain_support_details.present_modes.is_empty())
    }

    let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
    for physical_device in physical_devices {
        if device_supports_required_queues(instance, surface, surface_ptr, &physical_device)?
            && device_supports_required_extensions(instance, &physical_device)?
            && device_supports_swapchain(surface, surface_ptr, &physical_device)?
        {
            return Ok(physical_device);
        }
    }
    Err(anyhow!("no suitable graphics cards found!"))
}

fn find_queue_families(
    instance: &Instance,
    device: &PhysicalDevice,
    surface: &Surface,
    surface_ptr: &SurfaceKHR,
) -> Result<QueueFamilyIndicies> {
    fn find_queue_family_index(
        queue_family_properties: &Vec<QueueFamilyProperties>,
        flags: QueueFlags,
    ) -> Option<u32> {
        queue_family_properties
            .into_iter()
            .enumerate()
            .filter_map(|(index, queue_family_props)| {
                if queue_family_props.queue_flags.contains(flags) {
                    return Some(index as u32);
                } else {
                    return None;
                }
            })
            .collect::<Vec<_>>()
            .first()
            .cloned()
    }

    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(*device) };
    let graphics_family = find_queue_family_index(&queue_family_properties, QueueFlags::GRAPHICS);

    let mut present_family = None;
    for index in 0..queue_family_properties.len() as u32 {
        let supports_present =
            unsafe { surface.get_physical_device_surface_support(*device, index, *surface_ptr) }?;
        if supports_present {
            present_family = Some(index);
            break;
        }
    }

    Ok(QueueFamilyIndicies {
        graphics_family,
        present_family,
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

fn get_debug_utils_create_info<'a>() -> DebugUtilsMessengerCreateInfoEXTBuilder<'a> {
    DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            DebugUtilsMessageSeverityFlagsEXT::ERROR
                | DebugUtilsMessageSeverityFlagsEXT::WARNING
                | DebugUtilsMessageSeverityFlagsEXT::INFO
                | DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
        )
        .message_type(
            DebugUtilsMessageTypeFlagsEXT::GENERAL
                | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback))
}

fn create_debug_utils(
    entry: &Entry,
    instance: &Instance,
) -> Result<Option<(DebugUtils, DebugUtilsMessengerEXT)>> {
    if cfg!(debug_assertions) {
        let builder = get_debug_utils_create_info();
        let debug_utils = DebugUtils::new(entry, instance);
        let extension = unsafe { debug_utils.create_debug_utils_messenger(&builder, None)? };
        Ok(Some((debug_utils, extension)))
    } else {
        Ok(None)
    }
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: DebugUtilsMessageSeverityFlagsEXT,
    message_type: DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> Bool32 {
    let message = format!(
        "{:?}",
        std::ffi::CStr::from_ptr((*p_callback_data).p_message)
    );
    let ty = format!("{:?}", message_type).to_lowercase();

    match message_severity {
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            event!(Level::TRACE, message = message, ty = ty)
        }
        DebugUtilsMessageSeverityFlagsEXT::INFO => {
            event!(Level::INFO, message = message, ty = ty)
        }
        DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            event!(Level::WARN, message = message, ty = ty)
        }
        DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            event!(Level::ERROR, message = message, ty = ty)
        }
        _ => panic!(
            "Unknown message severity in vulkan_debug_utils_callback! {:?}",
            message_severity
        ),
    }
    // dont skip driver
    ash::vk::FALSE
}
