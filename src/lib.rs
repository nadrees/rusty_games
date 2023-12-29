use anyhow::Result;
use simple_logger::{set_up_color_terminal, SimpleLogger};

pub fn init_logging() -> Result<()> {
    set_up_color_terminal();
    let logger = SimpleLogger::new();
    logger.init()?;
    Ok(())
}

pub struct QueueFamilyIndicies {
    /// family capable of runing graphics related commands
    pub graphics_family: Option<u32>,
    /// family capable of displaying results on the screen
    pub present_family: Option<u32>,
}
