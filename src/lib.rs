mod graphics_pipeline;
mod logical_device;

use anyhow::Result;
pub use graphics_pipeline::create_graphics_pipeline;
pub use logical_device::create_logical_device;
use simple_logger::{set_up_color_terminal, SimpleLogger};

pub fn init_logging() -> Result<()> {
    set_up_color_terminal();
    let logger = SimpleLogger::new();
    logger.init()?;
    Ok(())
}
