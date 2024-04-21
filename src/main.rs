use std::ffi::{CStr, CString};

use anyhow::{anyhow, ensure, Result};
use ash::{
    ext::debug_utils,
    vk::{
        make_api_version, ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT,
        DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT,
        DeviceCreateInfo, DeviceQueueCreateInfo, InstanceCreateInfo, PhysicalDevice,
        PhysicalDeviceFeatures, Queue, QueueFlags, API_VERSION_1_3,
    },
    Device, Entry, Instance,
};
use glfw::{fail_on_errors, Glfw, PWindow};
use rusty_games::{init_logging, vulkan_debug_utils_callback};
use tracing::{debug, info, trace};

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
    queues: QueueHandles,
    /// The logical device for interfacing with the
    /// physical hardware
    device: Device,
    /// The debug utils extension, if enabled
    debug_utils: Option<DebugUtilsExt>,
    /// The instance for interacting with Vulkan core
    instance: Instance,
    // window must be dropped before glfw is,
    // do not move it before window in this list
    window: PWindow,
    glfw: Glfw,
}

impl App {
    pub fn new() -> Result<Self> {
        let (glfw, window) = Self::init_window()?;
        let (instance, debug_utils, device, queues) = Self::init_vulkan(&glfw)?;

        Ok(Self {
            debug_utils,
            device,
            queues,
            glfw,
            instance,
            window,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.main_loop()?;
        Ok(())
    }

    /// Creates the window that will interact with the OS to draw the results on the screen
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

    /// Initalizes Vulkan
    fn init_vulkan(glfw: &Glfw) -> Result<(Instance, Option<DebugUtilsExt>, Device, QueueHandles)> {
        let entry = Entry::linked();

        let instance = Self::create_instance(&entry, &glfw)?;
        let debug_utils = Self::setup_debug_messenger(&entry, &instance)?;
        let physical_device = Self::pick_physical_device(&instance)?;
        let (logical_device, queue_handles) =
            Self::create_logical_device(&instance, &physical_device)?;

        Ok((instance, debug_utils, logical_device, queue_handles))
    }

    /// Creates the logical device to interface with the selected physical device. Each queue family
    /// will create 1 queue instance for submitting commands to.
    fn create_logical_device(
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<(Device, QueueHandles)> {
        let indicies = Self::find_queue_families(instance, physical_device);
        ensure!(indicies.is_complete());

        let queue_priorities = [1.0f32];
        let device_queue_creation_info = [DeviceQueueCreateInfo::default()
            .queue_family_index(indicies.graphics_family.unwrap() as u32)
            .queue_priorities(&queue_priorities)];

        let physical_device_features = PhysicalDeviceFeatures::default();

        let device_create_info = DeviceCreateInfo::default()
            .queue_create_infos(&device_queue_creation_info)
            .enabled_features(&physical_device_features);

        let logical_device =
            unsafe { instance.create_device(*physical_device, &device_create_info, None) }?;

        let graphics_queue_handle =
            unsafe { logical_device.get_device_queue(indicies.graphics_family.unwrap() as u32, 0) };
        let queue_handles = QueueHandles {
            graphics: graphics_queue_handle,
        };

        Ok((logical_device, queue_handles))
    }

    /// Queries the Queue Families the physica device supports, and records the index of the relevant ones.
    fn find_queue_families(
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> QueueFamilyIndicies {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };
        QueueFamilyIndicies {
            _physical_device: *physical_device,
            graphics_family: queue_family_properties
                .iter()
                .position(|qfp| qfp.queue_flags.contains(QueueFlags::GRAPHICS)),
        }
    }

    /// Queries the system for the available physical devices, and picks the most appropriate one for use.
    fn pick_physical_device(instance: &Instance) -> Result<PhysicalDevice> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        let physical_device = physical_devices
            .into_iter()
            .find(|physical_device| Self::is_device_suitable(&instance, physical_device))
            .ok_or_else(|| anyhow!("Could not find a suitable physical device!"))?;
        let physical_device_props =
            unsafe { instance.get_physical_device_properties(physical_device) };
        trace!("Picked device {:?}", unsafe {
            CStr::from_ptr(physical_device_props.device_name.as_ptr())
        });
        return Ok(physical_device);
    }

    /// Returns if the specified physical device is suitable for use for this application.
    fn is_device_suitable(instance: &Instance, physical_device: &PhysicalDevice) -> bool {
        let indicies = Self::find_queue_families(instance, physical_device);
        indicies.is_complete()
    }

    /// Creates an Instance to interact with the core of Vulkan. Registers the needed extensions and
    /// layers, as well as basic information about the application.
    fn create_instance(entry: &Entry, glfw: &Glfw) -> Result<Instance> {
        let appname = CString::new(env!("CARGO_PKG_NAME"))?;
        let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>()?;
        let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u32>()?;
        let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u32>()?;
        let app_version = make_api_version(0, version_major, version_minor, version_patch);

        let app_info = ApplicationInfo::default()
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

        let instance_create_info = InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extension_name_ptrs)
            .enabled_layer_names(&enabled_layer_name_pts)
            .push_next(&mut debug_messenger_create_info);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        Ok(instance)
    }

    /// Returns the required layers needed for Vulkan. Notably, includes the validation
    /// layer if validations are enabled.
    fn gen_required_layers() -> Vec<String> {
        let mut layer_names = vec![];
        if ENABLE_VALIDATIONS {
            layer_names = vec!["VK_LAYER_KHRONOS_validation".to_owned()];
        }
        debug!("Layers to enable: {}", layer_names.join(", "));
        return layer_names;
    }

    /// Returns the needed instance exensions for Vulkan to function correctly.
    /// These always require the extensions necessary to interact with the native
    /// windowing system, and may include optional validation extensions if validations
    /// are enabled.
    fn get_required_instance_extensions(glfw: &Glfw) -> Result<Vec<String>> {
        let mut enabled_extension_names: Vec<String> = vec![];
        if let Some(glfw_extensions) = glfw.get_required_instance_extensions() {
            enabled_extension_names = glfw_extensions;
        }
        if ENABLE_VALIDATIONS {
            enabled_extension_names.push(debug_utils::NAME.to_str()?.to_owned());
        }
        debug!(
            "Instance extensions to enable: {}",
            enabled_extension_names.join(", ")
        );
        Ok(enabled_extension_names)
    }

    /// Configures the DebugUtils extension for which message types and severity levels to
    /// log.
    fn get_debug_messenger_create_info<'a>() -> DebugUtilsMessengerCreateInfoEXT<'a> {
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

    /// If validations are enabled, creates and registers the DebugUtils extension which prints
    /// logs to the console.
    fn setup_debug_messenger(entry: &Entry, instance: &Instance) -> Result<Option<DebugUtilsExt>> {
        if ENABLE_VALIDATIONS {
            let debug_utils_messenger_create_info = Self::get_debug_messenger_create_info();
            let debug_utils = debug_utils::Instance::new(entry, instance);
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
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

/// Struct for holding the needed references for the DebugUtils extension.
/// Primarily used so that we can correctly clean it up at application
/// exit.
struct DebugUtilsExt {
    debug_utils: debug_utils::Instance,
    extension: DebugUtilsMessengerEXT,
}

/// Holds the indexes of the relevant queue families for a given
/// physical device. Created from find_queue_families call.
struct QueueFamilyIndicies {
    /// Which physical device these queue families belong to
    _physical_device: PhysicalDevice,
    /// The graphics queue family index, if one is available
    graphics_family: Option<usize>,
}

impl QueueFamilyIndicies {
    /// True if all queue families are available for this physical
    /// device.
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some()
    }
}

/// Holds handles to the queues created as part of the logical
/// device initialization.
struct QueueHandles {
    graphics: Queue,
}
