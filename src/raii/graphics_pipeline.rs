use std::{ffi::CString, rc::Rc};

use crate::{
    logical_device::LogicalDeviceGuard,
    raii::{frame_buffer_guard::FrameBufferGuard, shader_module_guard::ShaderModuleGuard},
    SwapChainGuard,
};
use anyhow::Result;
use ash::vk::{
    CullModeFlags, FrontFace, GraphicsPipelineCreateInfo, Pipeline, PipelineCache,
    PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineLayoutCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
    PipelineViewportStateCreateInfo, PrimitiveTopology, Rect2D, SampleCountFlags, ShaderStageFlags,
    Viewport,
};
use tracing::debug;

use super::render_pass_guard::RenderPassGuard;

const VERTEX_SHADER_CODE: &str = "target/shaders/vert.spv";
const FRAGMENT_SHADER_CODE: &str = "target/shaders/frag.spv";

pub struct GraphicsPipeline {
    _frame_buffers: Vec<FrameBufferGuard>,
    logical_device: Rc<LogicalDeviceGuard>,
    pipeline_layout: PipelineLayout,
    pipeline: Pipeline,
    _render_pass: Rc<RenderPassGuard>,
}

impl GraphicsPipeline {
    pub fn try_new(
        render_pass: &Rc<RenderPassGuard>,
        subpass: u32,
        logical_device: &Rc<LogicalDeviceGuard>,
        swap_chain: &SwapChainGuard,
    ) -> Result<Self> {
        debug!("Creating graphics pipeline...");

        // configure shader moduels from .glsl files output
        let vertex_shader_module = ShaderModuleGuard::try_new(VERTEX_SHADER_CODE, logical_device)?;
        let fragment_shader_module =
            ShaderModuleGuard::try_new(FRAGMENT_SHADER_CODE, logical_device)?;
        let entry_point_name = CString::new("main")?;
        let shader_stages = [
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::VERTEX)
                .module(*vertex_shader_module)
                .name(&entry_point_name)
                .build(),
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::FRAGMENT)
                .module(*fragment_shader_module)
                .name(&entry_point_name)
                .build(),
        ];

        // vertex shader configuration to tell it how to get input data
        let vertex_input_state = PipelineVertexInputStateCreateInfo::builder();

        // how to process vertecies
        let input_assembly_state = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(PrimitiveTopology::TRIANGLE_LIST);

        // viewport and clipping (scissoring) settings
        let viewports = [Viewport::builder()
            .width(swap_chain.extent.width as f32)
            .height(swap_chain.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];
        let scissors = [Rect2D::builder().extent(swap_chain.extent).build()];
        let pipeline_viewport_state = PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        // rasterization settings
        let pipeline_rasterization_state = PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::CLOCKWISE);

        // multisampling settings
        let multisampling_state = PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1);

        // color blending to mix colors from previous fragment shader output and new
        // disabling this just takes the new output and passes it thru unchanged
        let color_blend_attachment_states = [PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .build()];
        let color_blend_state = PipelineColorBlendStateCreateInfo::builder()
            .attachments(&color_blend_attachment_states);

        // configure uniforms for the pipeline
        let pipeline_layout_info = PipelineLayoutCreateInfo::builder();
        let pipeline_layout =
            unsafe { logical_device.create_pipeline_layout(&pipeline_layout_info, None) }?;

        let pipeline_create_infos = [GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&pipeline_viewport_state)
            .rasterization_state(&pipeline_rasterization_state)
            .multisample_state(&multisampling_state)
            .color_blend_state(&color_blend_state)
            .layout(pipeline_layout)
            .render_pass(***render_pass)
            .subpass(subpass)
            .build()];

        let pipelines = unsafe {
            logical_device.create_graphics_pipelines(
                PipelineCache::null(),
                &pipeline_create_infos,
                None,
            )
        }
        .map_err(|(_, err)| err)?;

        debug!("Graphics pipeline created");

        let frame_buffers = swap_chain
            .image_views
            .iter()
            .map(|image_view| {
                FrameBufferGuard::try_new(image_view, &render_pass, &swap_chain, &logical_device)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(GraphicsPipeline {
            _frame_buffers: frame_buffers,
            pipeline: pipelines[0],
            pipeline_layout,
            logical_device: Rc::clone(logical_device),
            _render_pass: Rc::clone(render_pass),
        })
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        debug!("Dropping GraphicsPipeline");
        unsafe {
            self.logical_device.destroy_pipeline(self.pipeline, None);
            self.logical_device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
