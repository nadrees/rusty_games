mod instance;
mod logical_device;
mod physical_device_surface;
mod surface;

use std::ffi::CStr;

use anyhow::Result;
use ash::vk::{
    Bool32, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
    DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT, KHR_SWAPCHAIN_NAME,
};
pub use instance::Instance;
pub use logical_device::LogicalDevice;
pub use physical_device_surface::{PhysicalDeviceSurface, SwapChainSupportDetails};
use simple_logger::{set_up_color_terminal, SimpleLogger};
pub use surface::Surface;
use tracing::{event, Level};

const REQUIRED_DEVICE_EXTENSIONS: &[&CStr] = &[KHR_SWAPCHAIN_NAME];

pub fn init_logging() -> Result<()> {
    set_up_color_terminal();
    let logger = SimpleLogger::new();
    logger.init()?;
    Ok(())
}

pub unsafe extern "system" fn vulkan_debug_utils_callback(
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

/// Configures the DebugUtils extension for which message types and severity levels to
/// log.
pub fn get_debug_messenger_create_info<'a>() -> DebugUtilsMessengerCreateInfoEXT<'a> {
    DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | DebugUtilsMessageSeverityFlagsEXT::INFO
                | DebugUtilsMessageSeverityFlagsEXT::WARNING
                | DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            DebugUtilsMessageTypeFlagsEXT::GENERAL
                | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback))
}
