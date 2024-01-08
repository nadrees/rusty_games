mod logical_device;
mod raii;

use std::rc::Rc;

use anyhow::Result;
use ash::Entry;
use glfw::PWindow;
pub use logical_device::create_logical_device;
use logical_device::LogicalDeviceGuard;
use raii::graphics_pipeline::GraphicsPipeline;
use raii::render_pass_guard::RenderPassGuard;
pub use raii::swap_chain_guard::SwapChainGuard;
use simple_logger::{set_up_color_terminal, SimpleLogger};

pub fn init_logging() -> Result<()> {
    set_up_color_terminal();
    let logger = SimpleLogger::new();
    logger.init()?;
    Ok(())
}

pub fn create_swap_chain(
    entry: &Entry,
    logical_device: &Rc<LogicalDeviceGuard>,
    window: &PWindow,
) -> Result<SwapChainGuard> {
    SwapChainGuard::try_new(&entry, &logical_device, &window)
}

pub fn create_graphics_pipeline(
    logical_device: &Rc<LogicalDeviceGuard>,
    swap_chain: &SwapChainGuard,
) -> Result<GraphicsPipeline> {
    let render_pass = RenderPassGuard::try_new(logical_device, swap_chain)?;
    GraphicsPipeline::try_new(&render_pass, 0, logical_device, swap_chain)
}
