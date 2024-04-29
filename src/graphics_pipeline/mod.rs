mod pipeline_layout;
mod render_pass;

use anyhow::{ensure, Result};
use ash::vk::{
    ColorComponentFlags, CullModeFlags, FrontFace, GraphicsPipelineCreateInfo, Pipeline,
    PipelineCache, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineMultisampleStateCreateInfo,
    PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo,
    PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
    PrimitiveTopology, Rect2D, SampleCountFlags, ShaderModule, ShaderModuleCreateInfo,
    ShaderStageFlags, Viewport,
};
use std::{ffi::CStr, ops::Deref, rc::Rc};

use crate::{
    shaders::{FRAGMENT_SHADER_CODE, VERTEX_SHADER_CODE},
    LogicalDevice, Swapchain,
};

use self::{pipeline_layout::PipelineLayout, render_pass::RenderPass};

pub struct GraphicsPipeline {
    logical_device: Rc<LogicalDevice>,
    pipeline: Pipeline,
    render_pass: RenderPass,
    // references we need to keep to ensure we are cleaned up before
    // they are
    _pipeline_layout: PipelineLayout,
}

impl GraphicsPipeline {
    pub fn new(logical_device: &Rc<LogicalDevice>, swapchain: &Swapchain) -> Result<Self> {
        let shaders = create_shader_modules(logical_device)?;
        let pipeline_layout = PipelineLayout::new(logical_device)?;
        let render_pass = RenderPass::new(logical_device, swapchain)?;

        let shader_entrypoint_name = CStr::from_bytes_with_nul(b"main\0")?;
        let shader_stage_create_infos = shaders
            .into_iter()
            .map(|(shader_module, shader_stage)| {
                PipelineShaderStageCreateInfo::default()
                    .stage(shader_stage)
                    .module(shader_module)
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
        let swapchain_extent = *swapchain.get_extent();
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

        for (shader_module, _) in shaders {
            unsafe { logical_device.destroy_shader_module(shader_module, None) }
        }

        Ok(Self {
            logical_device: Rc::clone(logical_device),
            pipeline: graphics_pipeline[0],
            _pipeline_layout: pipeline_layout,
            render_pass,
        })
    }

    pub fn get_render_pass(&self) -> &RenderPass {
        &self.render_pass
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe { self.logical_device.destroy_pipeline(self.pipeline, None) }
    }
}

impl Deref for GraphicsPipeline {
    type Target = Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

/// Creates the shader modules and their associated pipeline create infos for use
/// in creating the graphics pipeline
fn create_shader_modules<'a>(
    logical_device: &Rc<LogicalDevice>,
) -> Result<[(ShaderModule, ShaderStageFlags); 2]> {
    let vertex_shader_code = VERTEX_SHADER_CODE;
    ensure!(
        vertex_shader_code.len() % 4 == 0,
        "Invalid vertex shader code read!"
    );
    let vertex_shader_module = create_shader_module(logical_device, vertex_shader_code)?;

    let fragment_shader_code = FRAGMENT_SHADER_CODE;
    ensure!(
        fragment_shader_code.len() % 4 == 0,
        "Invalid fragment shader code read!"
    );
    let fragment_shader_module = create_shader_module(logical_device, fragment_shader_code)?;

    Ok([
        (vertex_shader_module, ShaderStageFlags::VERTEX),
        (fragment_shader_module, ShaderStageFlags::FRAGMENT),
    ])
}

/// Reads in the raw bytes and creates a shader module from the read byte code
fn create_shader_module(logical_device: &Rc<LogicalDevice>, code: &[u8]) -> Result<ShaderModule> {
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
