use std::{
    collections::HashSet,
    ffi::{c_char, CStr, CString},
};

use anyhow::{anyhow, ensure, Result};
use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk::{
        self, make_api_version, ApplicationInfo, ColorSpaceKHR, ComponentMapping, ComponentSwizzle,
        CompositeAlphaFlagsKHR, CullModeFlags, DebugUtilsMessageSeverityFlagsEXT,
        DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT,
        DeviceCreateInfo, DeviceQueueCreateInfo, DynamicState, Extent2D, Format, FrontFace, Image,
        ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, ImageView, ImageViewCreateInfo,
        ImageViewType, InstanceCreateInfo, PhysicalDevice, PhysicalDeviceFeatures,
        PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
        PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineLayout,
        PipelineLayoutCreateInfo, PipelineMultisampleStateCreateInfo,
        PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo,
        PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
        PresentModeKHR, PrimitiveTopology, Queue, QueueFlags, Rect2D, ShaderModule,
        ShaderModuleCreateInfo, ShaderStageFlags, SharingMode, SurfaceCapabilitiesKHR,
        SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR, SwapchainKHR, Viewport,
        API_VERSION_1_3, KHR_SWAPCHAIN_NAME,
    },
    Device, Entry, Instance,
};
use rusty_games::{init_logging, vulkan_debug_utils_callback};
use tracing::{debug, info, trace};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Window, WindowBuilder},
};

const API_VERSION: u32 = API_VERSION_1_3;
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
    _queues: QueueHandles,
    /// The logical device for interfacing with the
    /// physical hardware
    device: Device,
    /// The debug utils extension, if enabled
    debug_utils: Option<DebugUtilsExt>,
    /// The instance for interacting with Vulkan core
    instance: Instance,
    /// The actual window presented to the user
    /// Need to keep a reference to this for the life
    /// off the app or it will get cleaned up
    _window: Window,
    /// See surface manager struct docs
    surface_manager: SurfaceManager,
    /// The linkage to the DLL for vulkan
    _entry: Entry,
    /// See swapchain manager struct docs
    swapchain_manager: SwapChainManager,
    /// Images from the swap chain
    _images: Vec<Image>,
    /// Views to interact with the images
    image_views: Vec<ImageView>,
    /// layout for dynamic variables within the graphics pipeline
    pipeline_layout: PipelineLayout,
}

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let required_extensions =
            ash_window::enumerate_required_extensions(event_loop.display_handle()?.as_raw())?;

        let window = Self::init_window(&event_loop)?;

        // init vulkan
        let entry = Entry::linked();
        let instance = Self::create_instance(&entry, required_extensions)?;
        let debug_utils = Self::setup_debug_messenger(&entry, &instance)?;
        let surface_manager = Self::create_surface_manager(&entry, &instance, &window)?;
        let physical_device = Self::pick_physical_device(&instance, &surface_manager)?;
        let (logical_device, queue_handles) =
            Self::create_logical_device(&instance, &physical_device, &surface_manager)?;
        let swapchain_manager = Self::create_swap_chain(
            &instance,
            &physical_device,
            &surface_manager,
            &window,
            &logical_device,
        )?;
        let images = swapchain_manager.get_swapchain_images()?;
        let image_views = Self::create_image_views(&logical_device, &swapchain_manager, &images)?;

        // configure graphics pipeline
        let shader_modules = Self::create_shader_modules(&logical_device)?;
        let pipeline_layout = Self::create_pipeline_layout(&logical_device)?;

        for (shader_module, _) in shader_modules {
            unsafe { logical_device.destroy_shader_module(shader_module, None) }
        }

        Ok(Self {
            _entry: entry,
            debug_utils,
            device: logical_device,
            _queues: queue_handles,
            instance,
            _window: window,
            surface_manager,
            swapchain_manager,
            _images: images,
            image_views,
            pipeline_layout,
        })
    }

    pub fn run(&mut self, event_loop: EventLoop<()>) -> Result<()> {
        event_loop.run(move |event, elwp| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id: _,
            } => {
                elwp.exit();
            }
            _ => {}
        })?;
        Ok(())
    }

    /// Creates the window that will interact with the OS to draw the results on the screen
    fn init_window(event_loop: &EventLoop<()>) -> Result<Window> {
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::<u32>::from((WINDOW_WIDTH, WINDOW_HEIGHT)))
            .with_resizable(false)
            .with_active(true)
            .with_title(WINDOW_TITLE)
            .build(&event_loop)?;
        Ok(window)
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
    ) -> Result<[(ShaderModule, PipelineShaderStageCreateInfo<'a>); 2]> {
        let vertex_shader_code = include_bytes!("../target/shaders/vert.spv");
        ensure!(
            vertex_shader_code.len() % 4 == 0,
            "Invalid vertex shader code read!"
        );
        let vertex_shader_module = Self::create_shader_module(logical_device, vertex_shader_code)?;
        let pipeline_vertex_shader_stage_create_info = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&CStr::from_bytes_with_nul(b"main\0")?);

        let fragment_shader_code = include_bytes!("../target/shaders/frag.spv");
        ensure!(
            fragment_shader_code.len() % 4 == 0,
            "Invalid fragment shader code read!"
        );
        let fragment_shader_module =
            Self::create_shader_module(logical_device, fragment_shader_code)?;
        let pipeline_fragment_shader_stage_create_info = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(&CStr::from_bytes_with_nul(b"main\0")?);

        Ok([
            (
                vertex_shader_module,
                pipeline_vertex_shader_stage_create_info,
            ),
            (
                fragment_shader_module,
                pipeline_fragment_shader_stage_create_info,
            ),
        ])
    }

    /// Configures the fixed function stages and creates a graphics pipeline to start submitting commands to
    fn create_graphics_pipeline(
        swapchain_manager: &SwapChainManager,
        window: &Window,
    ) -> Result<()> {
        // we're not using vertex buffers, so just an empty object
        let pipeline_vertex_input_state_create_info = PipelineVertexInputStateCreateInfo::default();

        // configure the vertexes to be interpreted as a list of triangles
        let pipeline_input_assembly_state_create_info =
            PipelineInputAssemblyStateCreateInfo::default()
                .topology(PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false);

        // default viewport covering entire swapchain extent, no depth filtering
        let swapchain_extent = swapchain_manager.support_details.choose_swap_extent(window);
        let viewport = Viewport::default()
            .x(0.0f32)
            .y(0.0f32)
            .width(swapchain_extent.width as f32)
            .height(swapchain_extent.height as f32)
            .min_depth(0.0f32)
            .max_depth(1.0f32);

        // default scissor, doing nothing
        let scissor = Rect2D::default().extent(swapchain_extent);

        let viewport_create_info = PipelineViewportStateCreateInfo::default()
            .viewports(&[viewport])
            .scissors(&[scissor]);

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
        let multisampling_state_create_info =
            PipelineMultisampleStateCreateInfo::default().sample_shading_enable(false);

        // settings for color blending per framebuffer. disable this for now, resulting in color output
        // from vertex shader passing thru
        let color_blend_attachment_state =
            PipelineColorBlendAttachmentState::default().blend_enable(false);

        // settings for global color blending. disable this as well.
        let pipeline_color_blend_state = PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&[color_blend_attachment_state]);

        Ok(())
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
        physical_device: &PhysicalDevice,
        surface_manager: &SurfaceManager,
        window: &Window,
        logical_device: &Device,
    ) -> Result<SwapChainManager> {
        let queue_indicies = Self::find_queue_families(instance, physical_device, surface_manager);
        let queue_family_indicies = Vec::from_iter(HashSet::from([
            queue_indicies.graphics_family.unwrap() as u32,
            queue_indicies.present_family.unwrap() as u32,
        ]));

        let swap_chain_support = Self::query_swap_chain_support(physical_device, surface_manager)?;
        let surface_format = swap_chain_support.choose_swap_surface_format();
        let present_mode = swap_chain_support.choose_swap_present_mode();
        let extent = swap_chain_support.choose_swap_extent(window);
        let image_count = swap_chain_support.get_image_count();

        let mut swap_chain_creation_info = SwapchainCreateInfoKHR::default()
            .surface(surface_manager.surface.unwrap())
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
            support_details: swap_chain_support,
            swapchain,
        })
    }

    /// Queries for the details of what the swap chain supports given
    /// the physical device and surface
    fn query_swap_chain_support(
        physical_device: &PhysicalDevice,
        surface_manager: &SurfaceManager,
    ) -> Result<SwapChainSupportDetails> {
        let capabilities = unsafe {
            surface_manager
                .surface_fn
                .get_physical_device_surface_capabilities(
                    *physical_device,
                    surface_manager.surface.unwrap(),
                )?
        };
        let formats = unsafe {
            surface_manager
                .surface_fn
                .get_physical_device_surface_formats(
                    *physical_device,
                    surface_manager.surface.unwrap(),
                )?
        };
        let present_modes = unsafe {
            surface_manager
                .surface_fn
                .get_physical_device_surface_present_modes(
                    *physical_device,
                    surface_manager.surface.unwrap(),
                )?
        };

        Ok(SwapChainSupportDetails {
            capabilities,
            formats,
            present_modes,
        })
    }

    /// Creates a manager that can create and destroy surfaces to be rendered to
    fn create_surface_manager(
        entry: &Entry,
        instance: &Instance,
        window: &Window,
    ) -> Result<SurfaceManager> {
        let surface_fn = surface::Instance::new(entry, instance);
        let mut surface_manager = SurfaceManager {
            surface_fn,
            surface: None,
        };
        surface_manager.create_surface(entry, instance, window)?;
        Ok(surface_manager)
    }

    /// Creates the logical device to interface with the selected physical device. Each queue family
    /// will create 1 queue instance for submitting commands to.
    fn create_logical_device(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        surface_manager: &SurfaceManager,
    ) -> Result<(Device, QueueHandles)> {
        let indicies = Self::find_queue_families(instance, physical_device, surface_manager);
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

        let logical_device =
            unsafe { instance.create_device(*physical_device, &device_create_info, None) }?;

        let graphics_queue_handle =
            unsafe { logical_device.get_device_queue(indicies.graphics_family.unwrap() as u32, 0) };
        let present_queue_handle =
            unsafe { logical_device.get_device_queue(indicies.present_family.unwrap() as u32, 0) };
        let queue_handles = QueueHandles {
            _graphics: graphics_queue_handle,
            _present: present_queue_handle,
        };

        Ok((logical_device, queue_handles))
    }

    /// Queries the Queue Families the physica device supports, and records the index of the relevant ones.
    fn find_queue_families(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        surface_manager: &SurfaceManager,
    ) -> QueueFamilyIndicies {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };
        // let physical_device_present_support = surface.get_physical_device_surface_support(*physical_device, queue_family_index, surface)
        QueueFamilyIndicies {
            _physical_device: *physical_device,
            graphics_family: queue_family_properties
                .iter()
                .position(|qfp| qfp.queue_flags.contains(QueueFlags::GRAPHICS)),
            present_family: queue_family_properties.iter().enumerate().position(
                |(idx, _)| unsafe {
                    surface_manager
                        .surface_fn
                        .get_physical_device_surface_support(
                            *physical_device,
                            idx as u32,
                            surface_manager.surface.unwrap(),
                        )
                        .unwrap_or_default()
                },
            ),
        }
    }

    /// Queries the system for the available physical devices, and picks the most appropriate one for use.
    fn pick_physical_device(
        instance: &Instance,
        surface_manager: &SurfaceManager,
    ) -> Result<PhysicalDevice> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        let mut physical_device = None;
        for pd in physical_devices {
            if Self::is_device_suitable(instance, &pd, surface_manager)? {
                physical_device = Some(pd);
                break;
            }
        }
        let physical_device =
            physical_device.ok_or_else(|| anyhow!("Could not find a suitable physical device!"))?;
        return Ok(physical_device);
    }

    /// Returns if the specified physical device is suitable for use for this application.
    fn is_device_suitable(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        surface_manager: &SurfaceManager,
    ) -> Result<bool> {
        let indicies = Self::find_queue_families(instance, physical_device, surface_manager);
        let supports_extensions =
            Self::check_device_extensions_supported(instance, physical_device)?;

        let mut swap_chain_supported = false;
        if supports_extensions {
            let swap_chain_support =
                Self::query_swap_chain_support(physical_device, surface_manager)?;
            swap_chain_supported = !swap_chain_support.formats.is_empty()
                && !swap_chain_support.present_modes.is_empty();
        }

        Ok(indicies.is_complete() && supports_extensions && swap_chain_supported)
    }

    /// Checks to see if the physical device supports all required device extensions
    fn check_device_extensions_supported(
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<bool> {
        let device_extension_properties =
            unsafe { instance.enumerate_device_extension_properties(*physical_device)? };

        let mut device_extension_names = HashSet::new();
        for device_extension in device_extension_properties {
            let extension_name = device_extension.extension_name_as_c_str()?;
            device_extension_names.insert(extension_name.to_owned());
        }

        for required_extension in REQUIRED_DEVICE_EXTENSIONS {
            let required_extension_name: CString = (*required_extension).to_owned();
            if !device_extension_names.contains(&required_extension_name) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Creates an Instance to interact with the core of Vulkan. Registers the needed extensions and
    /// layers, as well as basic information about the application.
    fn create_instance(entry: &Entry, required_extensions: &[*const c_char]) -> Result<Instance> {
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

        let enabled_extension_names = Self::get_required_instance_extensions(required_extensions)?;

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
            .enabled_extension_names(&enabled_extension_names)
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
    fn get_required_instance_extensions(
        required_extensions: &[*const c_char],
    ) -> Result<Vec<*const c_char>> {
        let mut enabled_extension_names: Vec<*const c_char> = Vec::from(required_extensions);
        if ENABLE_VALIDATIONS {
            enabled_extension_names.push(debug_utils::NAME.as_ptr());
        }
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
                .destroy_pipeline_layout(self.pipeline_layout, None)
        }

        for image_view in &self.image_views {
            unsafe { self.device.destroy_image_view(*image_view, None) }
        }

        self.swapchain_manager.destroy_swapchain();
        self.surface_manager.destroy_surface();

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
    present_family: Option<usize>,
}

impl QueueFamilyIndicies {
    /// True if all queue families are available for this physical
    /// device.
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

/// Holds handles to the queues created as part of the logical
/// device initialization.
struct QueueHandles {
    _graphics: Queue,
    _present: Queue,
}

/// Struct for creating and managing surfaces
struct SurfaceManager {
    surface_fn: surface::Instance,
    surface: Option<SurfaceKHR>,
}

impl SurfaceManager {
    pub fn create_surface(
        &mut self,
        entry: &Entry,
        instance: &Instance,
        window: &Window,
    ) -> Result<()> {
        ensure!(
            self.surface.is_none(),
            "Cannot create a new surface, one already exists!"
        );
        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };
        trace!("Surface created");
        self.surface = Some(surface);
        Ok(())
    }

    pub fn destroy_surface(&mut self) {
        if let Some(surface) = self.surface.take() {
            unsafe { self.surface_fn.destroy_surface(surface, None) };
            trace!("Surface destroyed");
            self.surface = None;
        }
    }
}

/// Details about what features the swap chain supports
/// for a given surface
struct SwapChainSupportDetails {
    capabilities: SurfaceCapabilitiesKHR,
    /// The formats (color depth settings) available to use.
    formats: Vec<SurfaceFormatKHR>,
    present_modes: Vec<PresentModeKHR>,
}

impl SwapChainSupportDetails {
    /// Picks the preferential surface format to use from the available
    pub fn choose_swap_surface_format(&self) -> &SurfaceFormatKHR {
        let srgb_color_space_formats = self
            .formats
            .iter()
            .filter(|format| format.color_space == ColorSpaceKHR::SRGB_NONLINEAR)
            .collect::<Vec<_>>();
        if let Some(b8g8r8a8_format) = srgb_color_space_formats
            .iter()
            .find(|format| format.format == Format::B8G8R8A8_SRGB)
        {
            return *b8g8r8a8_format;
        } else if let Some(srbg_format) = srgb_color_space_formats.first() {
            return *srbg_format;
        } else {
            return self.formats.first().unwrap();
        }
    }

    /// Picks the preferential swap mode to use based on the available
    pub fn choose_swap_present_mode(&self) -> PresentModeKHR {
        // prefer mailbox, where if we can render faster than the screen can present
        // and the queue fills up, we'll replace the last image with the most up to
        // date version
        if self.present_modes.contains(&PresentModeKHR::MAILBOX) {
            return PresentModeKHR::MAILBOX;
        }
        // otherwise, use FIFO - basically vertical sync. This is the only setting
        // guaranteed to be available on all systems
        return PresentModeKHR::FIFO;
    }

    /// Returns the "extent" of the images to draw - the resolution to use *in pixels*.
    pub fn choose_swap_extent(&self, window: &Window) -> Extent2D {
        match self.capabilities.current_extent.width {
            // in this scenario, we're in a high DPI setting where extent is in screen
            // space, but we need it to be in pixels. set it to the same size as the
            // window
            u32::MAX => {
                let window_size = window.inner_size();
                Extent2D {
                    width: window_size.width.clamp(
                        self.capabilities.min_image_extent.width,
                        self.capabilities.max_image_extent.width,
                    ),
                    height: window_size.height.clamp(
                        self.capabilities.min_image_extent.height,
                        self.capabilities.max_image_extent.height,
                    ),
                }
            }
            _ => self.capabilities.current_extent,
        }
    }

    /// Returns how many images the swap chain should use based on its support
    pub fn get_image_count(&self) -> u32 {
        let max_image_count = self.capabilities.max_image_count;
        let min_image_count = self.capabilities.min_image_count;

        let image_count = match max_image_count {
            // zero means there is no max, so just use 1 more than the minimum
            0 => min_image_count + 1,
            // in this case, use a few more than the minimum so we're not
            // stuck waiting on internal details
            _ => {
                let delta = (max_image_count - min_image_count) / 2;
                min_image_count + delta
            }
        };

        image_count.clamp(min_image_count, max_image_count)
    }
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
}
