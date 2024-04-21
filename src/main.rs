use std::ffi::CString;

use anyhow::{anyhow, Result};
use ash::{
    extensions::ext::DebugUtils,
    vk::{
        make_api_version, ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT,
        DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCreateInfoEXT,
        DebugUtilsMessengerCreateInfoEXTBuilder, DebugUtilsMessengerEXT, InstanceCreateInfo,
        PhysicalDevice, API_VERSION_1_3,
    },
    Entry, Instance,
};
use glfw::{fail_on_errors, Glfw, PWindow};
use rusty_games::{init_logging, vulkan_debug_utils_callback};
use tracing::{debug, info};

const API_VERSION: u32 = API_VERSION_1_3;
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_TITLE: &str = "Hello, Triangle";

#[cfg(feature = "enable_validations")]
const ENABLE_VALIDATIONS: bool = true;
#[cfg(not(feature = "enable_validations"))]
const ENABLE_VALIDATIONS: bool = false;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let mut app = App::new()?;
    app.run()?;

    Ok(())
}

struct App {
    debug_utils: Option<DebugUtilsExt>,
    instance: Instance,
    // window must be dropped before glfw is,
    // do not move it before window in this list
    window: PWindow,
    glfw: Glfw,
}

struct DebugUtilsExt {
    debug_utils: DebugUtils,
    extension: DebugUtilsMessengerEXT,
}

impl App {
    pub fn new() -> Result<Self> {
        let (glfw, window) = Self::init_window()?;
        let (instance, debug_utils) = Self::init_vulkan(&glfw)?;

        Ok(Self {
            debug_utils,
            glfw,
            instance,
            window,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.main_loop()?;
        Ok(())
    }

    fn init_window() -> Result<(Glfw, PWindow)> {
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
        Ok((glfw, window))
    }

    fn init_vulkan(glfw: &Glfw) -> Result<(Instance, Option<DebugUtilsExt>)> {
        let entry = Entry::linked();

        let instance = Self::create_instance(&entry, &glfw)?;
        let debug_utils = Self::setup_debug_messenger(&entry, &instance)?;
        Self::pick_physical_device(&instance)?;

        Ok((instance, debug_utils))
    }

    fn pick_physical_device(instance: &Instance) -> Result<PhysicalDevice> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        physical_devices
            .into_iter()
            .find(Self::is_device_suitable)
            .ok_or_else(|| anyhow!("Could not find a suitable physical device!"))
    }

    fn is_device_suitable(_physical_device: &PhysicalDevice) -> bool {
        return true;
    }

    fn create_instance(entry: &Entry, glfw: &Glfw) -> Result<Instance> {
        let appname = CString::new(env!("CARGO_PKG_NAME"))?;
        let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>()?;
        let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u32>()?;
        let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u32>()?;
        let app_version = make_api_version(0, version_major, version_minor, version_patch);

        let app_info = ApplicationInfo::builder()
            .application_name(&appname)
            .application_version(app_version)
            .api_version(API_VERSION)
            .engine_name(&appname)
            .engine_version(app_version);

        let enabled_extension_names = Self::get_required_instance_extensions(&glfw)?
            .into_iter()
            .map(|name| CString::new(name))
            .collect::<Result<Vec<_>, _>>()?;
        let enabled_extension_name_ptrs = enabled_extension_names
            .iter()
            .map(|extension| extension.as_ptr())
            .collect::<Vec<_>>();

        let enabled_layer_names = Self::gen_required_layers()
            .into_iter()
            .map(|layer_name| CString::new(layer_name))
            .collect::<Result<Vec<_>, _>>()?;
        let enabled_layer_name_pts = enabled_layer_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect::<Vec<_>>();

        let mut debug_messenger_create_info = Self::get_debug_messenger_create_info();

        let instance_create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extension_name_ptrs)
            .enabled_layer_names(&enabled_layer_name_pts)
            .push_next(&mut debug_messenger_create_info);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        Ok(instance)
    }

    fn gen_required_layers() -> Vec<String> {
        let mut layer_names = vec![];
        if ENABLE_VALIDATIONS {
            layer_names = vec!["VK_LAYER_KHRONOS_validation".to_owned()];
        }
        debug!("Layers to enable: {}", layer_names.join(", "));
        return layer_names;
    }

    fn get_required_instance_extensions(glfw: &Glfw) -> Result<Vec<String>> {
        let mut enabled_extension_names: Vec<String> = vec![];
        if let Some(glfw_extensions) = glfw.get_required_instance_extensions() {
            enabled_extension_names = glfw_extensions;
        }
        if ENABLE_VALIDATIONS {
            enabled_extension_names.push(DebugUtils::name().to_str()?.to_owned());
        }
        debug!(
            "Instance extensions to enable: {}",
            enabled_extension_names.join(", ")
        );
        Ok(enabled_extension_names)
    }

    fn get_debug_messenger_create_info<'a>() -> DebugUtilsMessengerCreateInfoEXTBuilder<'a> {
        DebugUtilsMessengerCreateInfoEXT::builder()
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

    fn setup_debug_messenger(entry: &Entry, instance: &Instance) -> Result<Option<DebugUtilsExt>> {
        if ENABLE_VALIDATIONS {
            let debug_utils_messenger_create_info = Self::get_debug_messenger_create_info();
            let debug_utils = DebugUtils::new(entry, instance);
            let extension = unsafe {
                debug_utils
                    .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)?
            };
            return Ok(Some(DebugUtilsExt {
                debug_utils,
                extension,
            }));
        }
        Ok(None)
    }

    fn main_loop(&mut self) -> Result<()> {
        while !self.window.should_close() {
            self.glfw.poll_events();
        }

        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        info!("Window closed, shutting down");

        if let Some(debug_utils) = &self.debug_utils {
            unsafe {
                debug_utils
                    .debug_utils
                    .destroy_debug_utils_messenger(debug_utils.extension, None)
            };
        }

        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
