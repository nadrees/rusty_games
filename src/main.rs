use std::{collections::HashSet, ffi::CStr, rc::Rc};

use anyhow::{anyhow, ensure, Result};
use ash::{
    ext::debug_utils,
    khr::swapchain,
    vk::{
        self, AccessFlags, AttachmentDescription, AttachmentLoadOp, AttachmentReference,
        AttachmentStoreOp, ClearColorValue, ClearValue, ColorComponentFlags, CommandBuffer,
        CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel,
        CommandBufferResetFlags, CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo,
        ComponentMapping, ComponentSwizzle, CompositeAlphaFlagsKHR, CullModeFlags,
        DebugUtilsMessengerEXT, DeviceCreateInfo, DeviceQueueCreateInfo, Fence, FenceCreateFlags,
        FenceCreateInfo, Framebuffer, FramebufferCreateInfo, FrontFace, GraphicsPipelineCreateInfo,
        Image, ImageAspectFlags, ImageLayout, ImageSubresourceRange, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, PhysicalDeviceFeatures, Pipeline, PipelineBindPoint,
        PipelineCache, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
        PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineLayoutCreateInfo,
        PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
        PipelineShaderStageCreateInfo, PipelineStageFlags, PipelineVertexInputStateCreateInfo,
        PipelineViewportStateCreateInfo, PolygonMode, PresentInfoKHR, PrimitiveTopology, Queue,
        Rect2D, RenderPass, RenderPassBeginInfo, RenderPassCreateInfo, SampleCountFlags, Semaphore,
        SemaphoreCreateInfo, ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags, SharingMode,
        SubmitInfo, SubpassContents, SubpassDependency, SubpassDescription, SwapchainCreateInfoKHR,
        SwapchainKHR, Viewport, KHR_SWAPCHAIN_NAME, SUBPASS_EXTERNAL,
    },
    Device, Entry,
};
use rusty_games::{
    get_debug_messenger_create_info, init_logging, Instance, PhysicalDeviceSurface, Surface,
    SwapChainSupportDetails,
};
use tracing::info;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::HasDisplayHandle,
    window::{Window, WindowBuilder, WindowButtons},
};

const REQUIRED_DEVICE_EXTENSIONS: &[&CStr] = &[KHR_SWAPCHAIN_NAME];
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
    /// Handles to the queues for submitting instructions to
    queues: QueueHandles,
    /// The logical device for interfacing with the
    /// physical hardware
    device: Device,
    /// The debug utils extension, if enabled
    debug_utils: Option<DebugUtilsExt>,
    /// The actual window presented to the user
    /// Need to keep a reference to this for the life
    /// off the app or it will get cleaned up
    window: Window,
    _pds: PhysicalDeviceSurface,
    /// The linkage to the DLL for vulkan
    _entry: Entry,
    /// See swapchain manager struct docs
    swapchain_manager: SwapChainManager,
    /// Images from the swap chain
    _images: Vec<Image>,
    /// Views to interact with the images
    image_views: Vec<ImageView>,
    /// render pass configuration for graphics pipeline
    render_pass: RenderPass,
    /// layout for dynamic variables within the graphics pipeline
    pipeline_layout: PipelineLayout,
    /// The graphics pipeline itself
    pipeline: Pipeline,
    /// The frame buffers for use in rendering images
    frame_buffers: Vec<Framebuffer>,
    /// Command pool responsible for managing memory and creating
    /// command buffers
    command_pool: CommandPool,
    /// The command buffer to submit draw commands to
    command_buffer: CommandBuffer,
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

        let window = Self::init_window(&event_loop)?;

        // init vulkan
        let entry = Entry::linked();
        let instance = Rc::new(Instance::new(&entry, required_extensions)?);
        let debug_utils = Self::setup_debug_messenger(&entry, &instance)?;
        let surface = Surface::new(&entry, &instance, &window)?;
        let physical_device_surface = Self::pick_physical_device(&instance, &Rc::new(surface))?;

        // TODO: move these
        let (logical_device, queue_handles) =
            Self::create_logical_device(&instance, &physical_device_surface)?;
        let swapchain_manager = Self::create_swap_chain(
            &instance,
            &physical_device_surface,
            &window,
            &logical_device,
        )?;
        let images = swapchain_manager.get_swapchain_images()?;
        let image_views = Self::create_image_views(&logical_device, &swapchain_manager, &images)?;

        // configure graphics pipeline
        let shader_modules = Self::create_shader_modules(&logical_device)?;
        let pipeline_layout = Self::create_pipeline_layout(&logical_device)?;
        let render_pass = Self::create_render_pass(&logical_device, &swapchain_manager)?;
        let pipeline = Self::create_graphics_pipeline(
            &logical_device,
            &pipeline_layout,
            &render_pass,
            &shader_modules,
            &swapchain_manager,
            &window,
        )?;

        for (shader_module, _) in shader_modules {
            unsafe { logical_device.destroy_shader_module(shader_module, None) }
        }

        let frame_buffers = Self::create_frame_buffers(
            &logical_device,
            &image_views,
            &render_pass,
            &swapchain_manager,
            &window,
        )?;

        // configure command buffers
        let command_pool = Self::create_command_pool(&logical_device, &physical_device_surface)?;
        let command_buffer = Self::create_command_buffer(&logical_device, &command_pool)?;
        let (image_available_semaphore, render_finished_semaphore, in_flight_fence) =
            Self::create_sync_object(&&logical_device)?;

        Ok(Self {
            _entry: entry,
            debug_utils,
            device: logical_device,
            queues: queue_handles,
            window,
            _pds: physical_device_surface,
            swapchain_manager,
            _images: images,
            image_views,
            render_pass,
            pipeline_layout,
            pipeline,
            frame_buffers,
            command_pool,
            command_buffer,
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
            .swapchain_manager
            .acquire_next_image_index(&self.image_available_semaphore)?;

        unsafe {
            self.device
                .reset_command_buffer(self.command_buffer, CommandBufferResetFlags::empty())?
        }

        self.record_command_buffer(image_index as usize)?;

        let wait_semaphores = [self.image_available_semaphore];
        let signal_semaphores = [self.render_finished_semaphore];
        let pipeline_stage_flags = [PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [self.command_buffer];
        let submit_info = [SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&pipeline_stage_flags)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)];
        unsafe {
            self.device
                .queue_submit(self.queues.graphics, &submit_info, self.in_flight_fence)?
        }

        let swapchains = [self.swapchain_manager.swapchain];
        let image_indicies = [image_index];
        let present_info = PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indicies);
        unsafe {
            self.swapchain_manager
                .device
                .queue_present(self.queues.present, &present_info)?
        };

        Ok(())
    }

    /// Records the command buffer for execution
    fn record_command_buffer(&self, image_index: usize) -> Result<()> {
        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        unsafe {
            self.device
                .begin_command_buffer(self.command_buffer, &command_buffer_begin_info)?
        };

        let swapchain_extent = self
            .swapchain_manager
            .support_details
            .choose_swap_extent(&self.window);
        let render_area = Rect2D::default().extent(swapchain_extent);

        let mut clear_value = ClearValue::default();
        clear_value.color = ClearColorValue {
            uint32: [0, 0, 0, 1],
        };
        let clear_values = [clear_value];

        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.frame_buffers[image_index])
            .render_area(render_area)
            .clear_values(&clear_values);
        unsafe {
            self.device.cmd_begin_render_pass(
                self.command_buffer,
                &render_pass_begin_info,
                SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                self.command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            self.device.cmd_draw(self.command_buffer, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(self.command_buffer);
            self.device.end_command_buffer(self.command_buffer)?;
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

    /// Allocates the command buffer from the command pool
    fn create_command_buffer(
        logical_device: &Device,
        command_pool: &CommandPool,
    ) -> Result<CommandBuffer> {
        let allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(*command_pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = unsafe { logical_device.allocate_command_buffers(&allocate_info)? };
        Ok(command_buffers[0])
    }

    /// Creates a command pool for getting the command buffers from
    fn create_command_pool(
        logical_device: &Device,
        physical_device_surface: &PhysicalDeviceSurface,
    ) -> Result<CommandPool> {
        let queue_family_indicies = physical_device_surface.get_queue_family_indicies();

        let create_command_pool = CommandPoolCreateInfo::default()
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_indicies.graphics_family.unwrap() as u32);
        let command_pool =
            unsafe { logical_device.create_command_pool(&create_command_pool, None)? };
        Ok(command_pool)
    }

    /// Creates the frame buffers
    fn create_frame_buffers(
        logical_device: &Device,
        image_views: &Vec<ImageView>,
        render_pass: &RenderPass,
        swapchain_manager: &SwapChainManager,
        window: &Window,
    ) -> Result<Vec<Framebuffer>> {
        let swapchain_extent = swapchain_manager.support_details.choose_swap_extent(window);
        let frame_buffers = image_views
            .iter()
            .map(|image_view| {
                let attachments = [*image_view];
                let create_info = FramebufferCreateInfo::default()
                    .render_pass(*render_pass)
                    .attachments(&attachments)
                    .height(swapchain_extent.height)
                    .width(swapchain_extent.width)
                    .layers(1);
                let framebuffer = unsafe { logical_device.create_framebuffer(&create_info, None)? };
                Ok::<_, vk::Result>(framebuffer)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(frame_buffers)
    }

    /// Configures the attachments and subpasses in the render pass
    fn create_render_pass(
        logical_device: &Device,
        swapchain_manager: &SwapChainManager,
    ) -> Result<RenderPass> {
        let attachment_description = [AttachmentDescription::default()
            // ensure attachment format matches that of swapchain
            .format(
                swapchain_manager
                    .support_details
                    .choose_swap_surface_format()
                    .format,
            )
            // not using multisampling, so stick to 1 sample
            .samples(SampleCountFlags::TYPE_1)
            // clear the data in the attachment before rendering
            .load_op(AttachmentLoadOp::CLEAR)
            // dont care about layout of previous image, because we're clearing it
            // anyway
            .initial_layout(ImageLayout::UNDEFINED)
            // store the results in memory for later user after rendering
            .store_op(AttachmentStoreOp::STORE)
            // transition to a layout suitable for presentation
            .final_layout(ImageLayout::PRESENT_SRC_KHR)
            // not using stencils
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)];

        let attachment_ref = [AttachmentReference::default()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

        let subpass_description = [SubpassDescription::default()
            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
            .color_attachments(&attachment_ref)];

        let subpass_dependencies = [SubpassDependency::default()
            .src_subpass(SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags::empty())
            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)];

        let render_pass_create_info = RenderPassCreateInfo::default()
            .attachments(&attachment_description)
            .subpasses(&subpass_description)
            .dependencies(&subpass_dependencies);

        let render_pass =
            unsafe { logical_device.create_render_pass(&render_pass_create_info, None)? };

        Ok(render_pass)
    }

    /// Creates the pipeline layout for passing dynamic values to the pipeline
    fn create_pipeline_layout(logical_device: &Device) -> Result<PipelineLayout> {
        let pipeline_layout_create_info = PipelineLayoutCreateInfo::default();
        let pipeline_layout =
            unsafe { logical_device.create_pipeline_layout(&pipeline_layout_create_info, None)? };
        Ok(pipeline_layout)
    }

    /// Creates the shader modules and their associated pipeline create infos for use
    /// in creating the graphics pipeline
    fn create_shader_modules<'a>(
        logical_device: &Device,
    ) -> Result<[(ShaderModule, ShaderStageFlags); 2]> {
        let vertex_shader_code = include_bytes!("../target/shaders/vert.spv");
        ensure!(
            vertex_shader_code.len() % 4 == 0,
            "Invalid vertex shader code read!"
        );
        let vertex_shader_module = Self::create_shader_module(logical_device, vertex_shader_code)?;

        let fragment_shader_code = include_bytes!("../target/shaders/frag.spv");
        ensure!(
            fragment_shader_code.len() % 4 == 0,
            "Invalid fragment shader code read!"
        );
        let fragment_shader_module =
            Self::create_shader_module(logical_device, fragment_shader_code)?;

        Ok([
            (vertex_shader_module, ShaderStageFlags::VERTEX),
            (fragment_shader_module, ShaderStageFlags::FRAGMENT),
        ])
    }

    /// Configures the fixed function stages and creates a graphics pipeline to start submitting commands to
    fn create_graphics_pipeline(
        logical_device: &Device,
        pipeline_layout: &PipelineLayout,
        render_pass: &RenderPass,
        shader_stages: &[(ShaderModule, ShaderStageFlags)],
        swapchain_manager: &SwapChainManager,
        window: &Window,
    ) -> Result<Pipeline> {
        let shader_entrypoint_name = CStr::from_bytes_with_nul(b"main\0")?;
        let shader_stage_create_infos = shader_stages
            .into_iter()
            .map(|(shader_module, shader_stage)| {
                PipelineShaderStageCreateInfo::default()
                    .stage(*shader_stage)
                    .module(*shader_module)
                    .name(&shader_entrypoint_name)
            })
            .collect::<Vec<_>>();

        // we're not using vertex buffers, so just an empty object
        let pipeline_vertex_input_state_create_info = PipelineVertexInputStateCreateInfo::default();

        // configure the vertexes to be interpreted as a list of triangles
        let pipeline_input_assembly_state_create_info =
            PipelineInputAssemblyStateCreateInfo::default()
                .topology(PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false);

        // default viewport covering entire swapchain extent, no depth filtering
        let swapchain_extent = swapchain_manager.support_details.choose_swap_extent(window);
        let viewport = [Viewport::default()
            .x(0.0f32)
            .y(0.0f32)
            .width(swapchain_extent.width as f32)
            .height(swapchain_extent.height as f32)
            .min_depth(0.0f32)
            .max_depth(1.0f32)];

        // default scissor, doing nothing
        let scissor = [Rect2D::default().extent(swapchain_extent)];

        let viewport_create_info = PipelineViewportStateCreateInfo::default()
            .viewports(&viewport)
            .scissors(&scissor);

        let rasteratization_create_info = PipelineRasterizationStateCreateInfo::default()
            // setting this to false discards points before the near plane or after the far plane
            // setting it to true would instead clamp them
            .depth_clamp_enable(false)
            // setting this to true would disable the rasterizer
            .rasterizer_discard_enable(false)
            // create filled polygons, instead of lines or points
            .polygon_mode(PolygonMode::FILL)
            // default line width
            .line_width(1.0f32)
            // culling will remove faces from the rasterization output
            // setting it to back removes the back faces
            .cull_mode(CullModeFlags::BACK)
            // determines how to know which face is front or back
            // in CLOCKWISE faces composed of verticies traveling in a clockwise direction are front facing
            .front_face(FrontFace::CLOCKWISE)
            // disable depth biasing, mainly used for shadow mapping
            .depth_bias_enable(false);

        // disable multisampling
        let multisampling_state_create_info = PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1);

        // settings for color blending per framebuffer. disable this for now, resulting in color output
        // from vertex shader passing thru
        let color_blend_attachment_state = [PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(ColorComponentFlags::RGBA)];

        // settings for global color blending. disable this as well.
        let pipeline_color_blend_state = PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachment_state);

        let graphics_pipeline_create_info = [GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&pipeline_vertex_input_state_create_info)
            .input_assembly_state(&pipeline_input_assembly_state_create_info)
            .render_pass(*render_pass)
            .color_blend_state(&pipeline_color_blend_state)
            .multisample_state(&multisampling_state_create_info)
            .viewport_state(&viewport_create_info)
            .rasterization_state(&rasteratization_create_info)
            .layout(*pipeline_layout)];

        let graphics_pipeline = unsafe {
            logical_device.create_graphics_pipelines(
                PipelineCache::null(),
                &graphics_pipeline_create_info,
                None,
            )
        }
        .map_err(|(_, r)| r)?;

        Ok(graphics_pipeline[0])
    }

    /// Reads in the raw bytes and creates a shader module from the read byte code
    fn create_shader_module(logical_device: &Device, code: &[u8]) -> Result<ShaderModule> {
        let code = code
            .chunks_exact(4)
            .map(|chunks| {
                let chunks = [chunks[0], chunks[1], chunks[2], chunks[3]];
                u32::from_ne_bytes(chunks)
            })
            .collect::<Vec<_>>();
        let shader_module_create_info = ShaderModuleCreateInfo::default().code(&code);
        let shader_module =
            unsafe { logical_device.create_shader_module(&shader_module_create_info, None)? };
        Ok(shader_module)
    }

    /// Creates Image views from the provided images
    fn create_image_views(
        logical_device: &Device,
        swapchain_manager: &SwapChainManager,
        images: &Vec<Image>,
    ) -> Result<Vec<ImageView>> {
        let format = swapchain_manager
            .support_details
            .choose_swap_surface_format();
        let image_views = images
            .iter()
            .map(|image| {
                let image_view_create_info = ImageViewCreateInfo::default()
                    .image(*image)
                    // 2D images
                    .view_type(ImageViewType::TYPE_2D)
                    .format(format.format)
                    // no swizzling
                    .components(
                        ComponentMapping::default()
                            .a(ComponentSwizzle::IDENTITY)
                            .b(ComponentSwizzle::IDENTITY)
                            .g(ComponentSwizzle::IDENTITY)
                            .r(ComponentSwizzle::IDENTITY),
                    )
                    // color images with no mipmapping or layers
                    .subresource_range(
                        ImageSubresourceRange::default()
                            .aspect_mask(ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    );
                let image_view =
                    unsafe { logical_device.create_image_view(&image_view_create_info, None)? };
                Ok::<_, vk::Result>(image_view)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(image_views)
    }

    /// Creates the swap chain used to present render images to the screen
    fn create_swap_chain(
        instance: &Instance,
        physical_device_surface: &PhysicalDeviceSurface,
        window: &Window,
        logical_device: &Device,
    ) -> Result<SwapChainManager> {
        let queue_indicies = physical_device_surface.get_queue_family_indicies();
        let queue_family_indicies = Vec::from_iter(HashSet::from([
            queue_indicies.graphics_family.unwrap() as u32,
            queue_indicies.present_family.unwrap() as u32,
        ]));

        let swap_chain_support = physical_device_surface.get_swapchain_support_details();
        let surface_format = swap_chain_support.choose_swap_surface_format();
        let present_mode = swap_chain_support.choose_swap_present_mode();
        let extent = swap_chain_support.choose_swap_extent(window);
        let image_count = swap_chain_support.get_image_count();

        let mut swap_chain_creation_info = SwapchainCreateInfoKHR::default()
            .surface(***physical_device_surface.get_surface())
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .present_mode(present_mode)
            // always 1 unless doing sterioscopic 3D
            .image_array_layers(1)
            // use images as color attachments for drawing color pictures to
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            // no transform
            .pre_transform(swap_chain_support.capabilities.current_transform)
            // ignore alpha channel
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            // enable clipping, to discard pixels that aren't visible
            .clipped(true)
            .old_swapchain(SwapchainKHR::null());
        if queue_family_indicies.len() == 1 {
            swap_chain_creation_info =
                swap_chain_creation_info.image_sharing_mode(SharingMode::EXCLUSIVE);
        } else {
            swap_chain_creation_info = swap_chain_creation_info
                .image_sharing_mode(SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indicies);
        }

        let swapchain_device = swapchain::Device::new(instance, logical_device);
        let swapchain =
            unsafe { swapchain_device.create_swapchain(&swap_chain_creation_info, None) }?;

        Ok(SwapChainManager {
            device: swapchain_device,
            support_details: swap_chain_support.clone(),
            swapchain,
        })
    }

    /// Creates the logical device to interface with the selected physical device. Each queue family
    /// will create 1 queue instance for submitting commands to.
    fn create_logical_device(
        instance: &Instance,
        physical_device_surface: &PhysicalDeviceSurface,
    ) -> Result<(Device, QueueHandles)> {
        let indicies = physical_device_surface.get_queue_family_indicies();
        ensure!(indicies.is_complete());

        let unique_queue_family_indicies = HashSet::from([
            indicies.graphics_family.unwrap() as u32,
            indicies.present_family.unwrap() as u32,
        ]);

        let queue_priorities = [1.0f32];
        let device_queue_creation_infos = unique_queue_family_indicies
            .into_iter()
            .map(|queue_family_index| {
                DeviceQueueCreateInfo::default()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&queue_priorities)
            })
            .collect::<Vec<_>>();

        let physical_device_features = PhysicalDeviceFeatures::default();

        let extension_names = REQUIRED_DEVICE_EXTENSIONS
            .iter()
            .map(|extension_name| (**extension_name).as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = DeviceCreateInfo::default()
            .queue_create_infos(&device_queue_creation_infos)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&extension_names);

        let logical_device = unsafe {
            instance.create_device(
                physical_device_surface.get_physical_device(),
                &device_create_info,
                None,
            )
        }?;

        let graphics_queue_handle =
            unsafe { logical_device.get_device_queue(indicies.graphics_family.unwrap() as u32, 0) };
        let present_queue_handle =
            unsafe { logical_device.get_device_queue(indicies.present_family.unwrap() as u32, 0) };
        let queue_handles = QueueHandles {
            graphics: graphics_queue_handle,
            present: present_queue_handle,
        };

        Ok((logical_device, queue_handles))
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
    fn setup_debug_messenger(entry: &Entry, instance: &Instance) -> Result<Option<DebugUtilsExt>> {
        if ENABLE_VALIDATIONS {
            let debug_utils_messenger_create_info = get_debug_messenger_create_info();
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
            self.device.destroy_command_pool(self.command_pool, None);
        }

        for frame_buffer in &self.frame_buffers {
            unsafe { self.device.destroy_framebuffer(*frame_buffer, None) }
        }

        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }

        for image_view in &self.image_views {
            unsafe { self.device.destroy_image_view(*image_view, None) }
        }

        self.swapchain_manager.destroy_swapchain();

        unsafe {
            self.device.destroy_device(None);
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

/// Holds handles to the queues created as part of the logical
/// device initialization.
struct QueueHandles {
    graphics: Queue,
    present: Queue,
}

/// Struct for holding references to swapchain
struct SwapChainManager {
    device: swapchain::Device,
    support_details: SwapChainSupportDetails,
    swapchain: SwapchainKHR,
}

impl SwapChainManager {
    pub fn get_swapchain_images(&self) -> Result<Vec<Image>> {
        let images = unsafe { self.device.get_swapchain_images(self.swapchain)? };
        Ok(images)
    }

    pub fn destroy_swapchain(&mut self) {
        unsafe { self.device.destroy_swapchain(self.swapchain, None) }
    }

    /// Aquires the index of the next image to use from the swapchain, and registers the
    /// signal semaphore to be signaled when its ready for use.
    pub fn acquire_next_image_index(&self, signal_semaphore: &Semaphore) -> Result<u32> {
        let (index, _) = unsafe {
            self.device.acquire_next_image(
                self.swapchain,
                u64::MAX,
                *signal_semaphore,
                Fence::null(),
            )?
        };
        Ok(index)
    }
}
