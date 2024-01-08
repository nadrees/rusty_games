use std::{ffi::CString, rc::Rc};

use crate::{raii::shader_module_guard::ShaderModuleGuard, LogicalDeviceGuard, SwapChainGuard};
use anyhow::Result;
use ash::vk::{
    CullModeFlags, FrontFace, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineLayoutCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
    PipelineViewportStateCreateInfo, PrimitiveTopology, Rect2D, ShaderStageFlags, Viewport,
};

const VERTEX_SHADER_CODE: &str = "target/shaders/vert.spv";
const FRAGMENT_SHADER_CODE: &str = "target/shaders/frag.spv";

pub struct GraphicsPipeline {
    logical_device: Rc<LogicalDeviceGuard>,
    pipeline_layout: PipelineLayout,
}

impl GraphicsPipeline {
    pub fn try_new(
        logical_device: &Rc<LogicalDeviceGuard>,
        swap_chain: &SwapChainGuard,
    ) -> Result<Self> {
        // configure shader moduels from .glsl files output
        let vertex_shader_module = ShaderModuleGuard::try_new(VERTEX_SHADER_CODE, logical_device)?;
        let fragment_shader_module =
            ShaderModuleGuard::try_new(FRAGMENT_SHADER_CODE, logical_device)?;
        let shader_stages = vec![
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::VERTEX)
                .module(*vertex_shader_module)
                .name(&CString::new("main")?),
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::FRAGMENT)
                .module(*fragment_shader_module)
                .name(&CString::new("main")?),
        ];

        // vertex shader configuration to tell it how to get input data
        let vertex_input_state = PipelineVertexInputStateCreateInfo::builder();

        // how to process vertecies
        let input_assembly_state = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(PrimitiveTopology::TRIANGLE_LIST);

        // viewport and clipping (scissoring) settings
        let viewport = Viewport::builder()
            .width(swap_chain.extent.width as f32)
            .height(swap_chain.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();
        let scissor = Rect2D::builder().extent(swap_chain.extent).build();
        let pipeline_viewport_state = PipelineViewportStateCreateInfo::builder()
            .viewports(&[viewport])
            .scissors(&[scissor]);

        // rasterization settings
        let pipeline_rasterization_state = PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::CLOCKWISE);

        // multisampling settings
        let multisampling_state =
            PipelineMultisampleStateCreateInfo::builder().sample_shading_enable(false);

        // color blending to mix colors from previous fragment shader output and new
        // disabling this just takes the new output and passes it thru unchanged
        let color_blend_attachment_state = PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .build();
        let color_blend_state = PipelineColorBlendStateCreateInfo::builder()
            .attachments(&[color_blend_attachment_state]);

        // configure uniforms for the pipeline
        let pipeline_layout_info = PipelineLayoutCreateInfo::builder();
        let pipeline_layout =
            unsafe { logical_device.create_pipeline_layout(&pipeline_layout_info, None) }?;

        Ok(GraphicsPipeline {
            pipeline_layout,
            logical_device: Rc::clone(logical_device),
        })
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_pipeline_layout(self.pipeline_layout, None)
        }
    }
}
