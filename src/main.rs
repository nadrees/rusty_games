use std::ffi::CString;

use anyhow::{anyhow, Result};
use ash::{
    extensions::ext::DebugUtils,
    vk::{
        make_api_version, ApplicationInfo, Bool32, DebugUtilsMessageSeverityFlagsEXT,
        DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT,
        DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerCreateInfoEXTBuilder,
        DebugUtilsMessengerEXT, DeviceCreateInfo, DeviceQueueCreateInfo, InstanceCreateInfo,
        PhysicalDevice, QueueFamilyProperties, QueueFlags, API_VERSION_1_3,
    },
    Entry, Instance,
};
use glfw::{fail_on_errors, Glfw};
use rusty_games::{init_logging, QueueFamilyIndicies};
use tracing::{event, Level};

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

    let extension_names: Vec<CString> = get_extension_names(&glfw)?
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

    let debug_utils = create_debug_utils(&entry, &instance)?;

    let physical_device = get_physical_device(&instance)?;
    let queue_family_indicies = find_queue_families(&instance, &physical_device);

    let queue_priorities = vec![1.0f32];
    let device_queue_create_infos = vec![DeviceQueueCreateInfo::builder()
        .queue_family_index(
            queue_family_indicies
                .graphics_family
                .ok_or(anyhow!("No graphics family index"))?,
        )
        .queue_priorities(&queue_priorities)
        .build()];
    let device_create_info =
        DeviceCreateInfo::builder().queue_create_infos(&device_queue_create_infos);
    let logical_device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };

    while !window.should_close() {
        glfw.wait_events();
    }

    if let Some((debug_utils, debug_utils_extension)) = debug_utils {
        unsafe { debug_utils.destroy_debug_utils_messenger(debug_utils_extension, None) }
    }
    unsafe {
        logical_device.destroy_device(None);
        instance.destroy_instance(None);
    };

    Ok(())
}

fn get_physical_device(instance: &Instance) -> Result<PhysicalDevice> {
    let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
    for physical_device in physical_devices {
        let queue_families = find_queue_families(instance, &physical_device);
        if queue_families.graphics_family.is_some() {
            return Ok(physical_device);
        }
    }
    Err(anyhow!("no suitable graphics cards found!"))
}

fn find_queue_families(instance: &Instance, device: &PhysicalDevice) -> QueueFamilyIndicies {
    fn find_queue_family_index(
        queue_family_properties: Vec<QueueFamilyProperties>,
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
    let graphics_family = find_queue_family_index(queue_family_properties, QueueFlags::GRAPHICS);
    QueueFamilyIndicies { graphics_family }
}

fn get_extension_names(glfw: &Glfw) -> Result<Vec<String>> {
    let mut extension_names: Vec<String> = vec![DebugUtils::name().to_str()?.to_owned()];
    if let Some(mut glfw_required_extensions) = glfw.get_required_instance_extensions() {
        extension_names.append(&mut glfw_required_extensions);
    }
    Ok(extension_names)
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
