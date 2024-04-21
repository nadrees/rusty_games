use std::ffi::CString;

use anyhow::{anyhow, Result};
use ash::{
    extensions::ext::DebugUtils,
    vk::{make_api_version, ApplicationInfo, InstanceCreateInfo, API_VERSION_1_3},
    Entry, Instance,
};
use glfw::{fail_on_errors, Glfw, PWindow};
use rusty_games::init_logging;
use tracing::info;

const API_VERSION: u32 = API_VERSION_1_3;
const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_TITLE: &str = "Hello, Triangle";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let mut app = App::new()?;
    app.run()?;

    Ok(())
}

struct App {
    instance: Instance,
    // window must be dropped before glfw is,
    // do not move it before window in this list
    window: PWindow,
    glfw: Glfw,
}

impl App {
    pub fn new() -> Result<Self> {
        let (glfw, window) = Self::init_window()?;
        let instance = Self::init_vulkan(&glfw)?;

        Ok(Self {
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

    fn init_vulkan(glfw: &Glfw) -> Result<Instance> {
        let entry = Entry::linked();

        let instance = Self::create_instance(&entry, &glfw)?;

        Ok(instance)
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

        let enabled_layer_names = VALIDATION_LAYERS
            .iter()
            .map(|layer_name| CString::new(*layer_name))
            .collect::<Result<Vec<_>, _>>()?;
        let enabled_layer_name_pts = enabled_layer_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect::<Vec<_>>();

        let instance_create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extension_name_ptrs)
            .enabled_layer_names(&enabled_layer_name_pts);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        Ok(instance)
    }

    fn get_required_instance_extensions(glfw: &Glfw) -> Result<Vec<String>> {
        let mut enabled_extension_names: Vec<String> = vec![];
        if let Some(glfw_extensions) = glfw.get_required_instance_extensions() {
            enabled_extension_names = glfw_extensions;
        }
        enabled_extension_names.push(DebugUtils::name().to_str()?.to_owned());
        Ok(enabled_extension_names)
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

        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
