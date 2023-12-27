use anyhow::Result;
use simple_logger::{set_up_color_terminal, SimpleLogger};

pub fn init() -> Result<()> {
    set_up_color_terminal();
    let logger = SimpleLogger::new();
    logger.init()?;
    Ok(())
}
