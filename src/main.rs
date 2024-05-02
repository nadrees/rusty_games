use std::{ffi::CStr, rc::Rc};

use anyhow::{anyhow, Result};
use ash::{
    ext::debug_utils,
    vk::{
        ClearColorValue, ClearValue, CommandBufferBeginInfo, CommandBufferResetFlags,
        DebugUtilsMessengerEXT, Fence, FenceCreateFlags, FenceCreateInfo, PipelineBindPoint,
        PipelineStageFlags, PresentInfoKHR, Rect2D, RenderPassBeginInfo, Semaphore,
        SemaphoreCreateInfo, SubmitInfo, SubpassContents,
    },
    Device, Entry,
};
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
    /// The graphics pipeline itself
    pipeline: GraphicsPipeline,
    /// Command pool responsible for managing memory and creating
    /// command buffers
    command_pool: CommandPool,
    /// Semaphore for when the image is available to be used from the
    /// swapchain
    image_available_semaphore: Semaphore,
    /// Semaphore for when the rendering has finished
    render_finished_semaphore: Semaphore,
    /// Fence for synchronizing render passes
    in_flight_fence: Fence,
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
        let command_pool = CommandPool::new(&logical_device)?;
        let (image_available_semaphore, render_finished_semaphore, in_flight_fence) =
            Self::create_sync_object(&&logical_device)?;

        Ok(Self {
            debug_utils,
            device: logical_device,
            swapchain,
            pipeline,
            command_pool,
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
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

    fn draw_frame(&self) -> Result<()> {
        let fences = [self.in_flight_fence];
        unsafe {
            // wait for previous draw to complete
            self.device.wait_for_fences(&fences, true, u64::MAX)?;
            // reset the fence so that it can be re-signaled when this draw is complete
            self.device.reset_fences(&fences)?;
        }

        let image_index = self
            .swapchain
            .acquire_next_image_index(&self.image_available_semaphore)?;

        let command_buffer = *self.command_pool.get_command_buffer();

        unsafe {
            self.device
                .reset_command_buffer(command_buffer, CommandBufferResetFlags::empty())?
        }

        self.record_command_buffer(image_index as usize)?;

        let wait_semaphores = [self.image_available_semaphore];
        let signal_semaphores = [self.render_finished_semaphore];
        let pipeline_stage_flags = [PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [command_buffer];
        let submit_info = [SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&pipeline_stage_flags)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)];
        unsafe {
            self.device.queue_submit(
                self.device.get_queues().graphics,
                &submit_info,
                self.in_flight_fence,
            )?
        }

        let swapchains = [*self.swapchain.get_handle()];
        let image_indicies = [image_index];
        let present_info = PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indicies);
        unsafe {
            self.swapchain
                .queue_present(self.device.get_queues().present, &present_info)?
        };

        Ok(())
    }

    /// Records the command buffer for execution
    fn record_command_buffer(&self, image_index: usize) -> Result<()> {
        let command_buffer = *self.command_pool.get_command_buffer();

        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)?
        };

        let swapchain_extent = self.swapchain.get_extent();
        let render_area = Rect2D::default().extent(*swapchain_extent);

        let mut clear_value = ClearValue::default();
        clear_value.color = ClearColorValue {
            uint32: [0, 0, 0, 1],
        };
        let clear_values = [clear_value];

        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(**self.pipeline.get_render_pass())
            .framebuffer(**self.pipeline.get_framebuffer_for_index(image_index))
            .render_area(render_area)
            .clear_values(&clear_values);
        unsafe {
            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                *self.pipeline,
            );
            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(command_buffer);
            self.device.end_command_buffer(command_buffer)?;
        };

        Ok(())
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

    fn create_sync_object(logical_device: &Device) -> Result<(Semaphore, Semaphore, Fence)> {
        let semaphore_create_info = SemaphoreCreateInfo::default();
        let fence_create_info = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);

        let image_availabe_semaphore =
            unsafe { logical_device.create_semaphore(&semaphore_create_info, None)? };
        let render_finished_semaphore =
            unsafe { logical_device.create_semaphore(&semaphore_create_info, None)? };
        let in_flight_fence = unsafe { logical_device.create_fence(&fence_create_info, None)? };

        Ok((
            image_availabe_semaphore,
            render_finished_semaphore,
            in_flight_fence,
        ))
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

        unsafe {
            self.device
                .destroy_semaphore(self.image_available_semaphore, None);
            self.device
                .destroy_semaphore(self.render_finished_semaphore, None);
            self.device.destroy_fence(self.in_flight_fence, None);
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
