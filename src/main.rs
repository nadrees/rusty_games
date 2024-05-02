use std::{ffi::CStr, rc::Rc};

use anyhow::{anyhow, Result};
use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT, Entry};
use rusty_games::{
    get_debug_messenger_create_info, init_logging, CommandPool, GraphicsPipeline, Instance,
    LogicalDevice, PhysicalDeviceSurface, Surface, Swapchain,
};
use tracing::info;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::HasDisplayHandle,
    window::{Window, WindowBuilder, WindowButtons},
};

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_TITLE: &str = "Hello, Triangle";

#[cfg(feature = "enable_validations")]
const ENABLE_VALIDATIONS: bool = true;
#[cfg(not(feature = "enable_validations"))]
const ENABLE_VALIDATIONS: bool = false;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let event_loop = EventLoop::new()?;
    let mut app = App::new(&event_loop)?;
    app.run(event_loop)?;

    Ok(())
}

struct App {
    /// The logical device for interfacing with the
    /// physical hardware
    device: Rc<LogicalDevice>,
    /// The debug utils extension, if enabled
    debug_utils: Option<DebugUtilsExt>,
    /// See swapchain manager struct docs
    swapchain: Swapchain,
    /// Command pool responsible for managing memory and creating
    /// command buffers
    command_pool: CommandPool,
}

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let required_extensions =
            ash_window::enumerate_required_extensions(event_loop.display_handle()?.as_raw())?
                .into_iter()
                .map(|extension| unsafe { CStr::from_ptr(*extension) }.to_str())
                .collect::<Result<Vec<_>, _>>()?;

        let window = Rc::new(Self::init_window(&event_loop)?);

        // init vulkan
        let entry = Entry::linked();
        let instance = Rc::new(Instance::new(entry, required_extensions)?);
        let debug_utils = Self::setup_debug_messenger(&instance)?;
        let surface = Surface::new(&instance, &window)?;
        let physical_device_surface = Self::pick_physical_device(&instance, &Rc::new(surface))?;
        let logical_device = Rc::new(TryInto::<LogicalDevice>::try_into(physical_device_surface)?);
        let swapchain = Swapchain::new(&instance, &window, &logical_device)?;

        // configure graphics pipeline
        let pipeline = GraphicsPipeline::new(&logical_device, &swapchain)?;

        // configure command buffers
        let command_pool = CommandPool::new(&logical_device, pipeline)?;

        Ok(Self {
            debug_utils,
            device: logical_device,
            swapchain,
            command_pool,
        })
    }

    pub fn run(&mut self, event_loop: EventLoop<()>) -> Result<()> {
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run(move |event, elwp| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id: _,
            } => {
                elwp.exit();
            }
            Event::AboutToWait => {
                self.draw_frame().unwrap();
            }
            Event::LoopExiting => {
                // wait for vulkan to finish up before exiting
                unsafe { self.device.device_wait_idle() }.unwrap();
            }
            _ => {}
        })?;
        Ok(())
    }

    fn draw_frame(&mut self) -> Result<()> {
        let frame = self.command_pool.get_next_frame();
        frame.render(&self.swapchain)
    }

    /// Creates the window that will interact with the OS to draw the results on the screen
    fn init_window(event_loop: &EventLoop<()>) -> Result<Window> {
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::<u32>::from((WINDOW_WIDTH, WINDOW_HEIGHT)))
            .with_resizable(false)
            .with_enabled_buttons(WindowButtons::CLOSE)
            .with_active(true)
            .with_title(WINDOW_TITLE)
            .build(&event_loop)?;
        Ok(window)
    }

    /// Queries the system for the available physical devices, and picks the most appropriate one for use.
    fn pick_physical_device(
        instance: &Rc<Instance>,
        surface: &Rc<Surface>,
    ) -> Result<PhysicalDeviceSurface> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        for pd in physical_devices {
            let pds = PhysicalDeviceSurface::new(instance, surface, pd)?;
            if pds.is_suitable()? {
                return Ok(pds);
            }
        }
        Err(anyhow!("Could not find a suitable physical device!"))
    }

    /// If validations are enabled, creates and registers the DebugUtils extension which prints
    /// logs to the console.
    fn setup_debug_messenger(instance: &Instance) -> Result<Option<DebugUtilsExt>> {
        if ENABLE_VALIDATIONS {
            let debug_utils_messenger_create_info = get_debug_messenger_create_info();
            let debug_utils = debug_utils::Instance::new(instance.get_entry(), instance);
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
    }
}

/// Struct for holding the needed references for the DebugUtils extension.
/// Primarily used so that we can correctly clean it up at application
/// exit.
struct DebugUtilsExt {
    debug_utils: debug_utils::Instance,
    extension: DebugUtilsMessengerEXT,
}
