mod frame_buffer_guard;
mod graphics_pipeline_guard;
mod image_view_guard;
mod render_pass_guard;
mod shader_module_guard;
mod swap_chain_guard;

use std::rc::Rc;

use anyhow::Result;
use ash::Entry;
use glfw::PWindow;

use crate::logical_device::LogicalDeviceGuard;

use self::{
    graphics_pipeline_guard::GraphicsPipelineGuard, render_pass_guard::RenderPassGuard,
    swap_chain_guard::SwapChainGuard,
};

pub fn create_graphics_pipeline(
    entry: &Entry,
    window: &PWindow,
    logical_device: &Rc<LogicalDeviceGuard>,
) -> Result<GraphicsPipelineGuard> {
    let swap_chain = SwapChainGuard::try_new(&entry, &logical_device, &window)?;
    let render_pass = RenderPassGuard::try_new(logical_device, &swap_chain)?;
    GraphicsPipelineGuard::try_new(&render_pass, 0, logical_device, &swap_chain)
}
