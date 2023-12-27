use ash::{
    extensions::ext::DebugUtils,
    vk::{
        Bool32, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
        DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT,
        DebugUtilsMessengerCreateInfoEXTBuilder, DebugUtilsMessengerEXT,
    },
    Entry, Instance,
};
use tracing::{event, Level};

use super::ExtensionImpl;

pub struct DebugUtilsGuard {
    debug_utils: DebugUtils,
    extension: DebugUtilsMessengerEXT,
}

impl DebugUtilsGuard {
    pub fn get_debug_create_info<'a>() -> DebugUtilsMessengerCreateInfoEXTBuilder<'a> {
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
}

impl ExtensionImpl for DebugUtilsGuard {
    fn name() -> String {
        DebugUtils::name().to_str().unwrap().to_owned()
    }

    fn try_new(entry: &Entry, instance: &Instance) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let debug_utils = DebugUtils::new(entry, instance);
        let debug_create_info = Self::get_debug_create_info();
        let extension =
            unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None)? };
        Ok(Self {
            debug_utils,
            extension,
        })
    }
}

impl Drop for DebugUtilsGuard {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils
                .destroy_debug_utils_messenger(self.extension, None)
        }
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
