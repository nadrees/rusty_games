pub mod physical_device;
pub mod queue_families;
mod raii;

use anyhow::Result;
pub use raii::debug_utils_extension::{get_debug_utils_create_info, DebugUtilsExtension};
pub use raii::instance_guard::InstanceGuard;
pub use raii::logical_device_guard::LogicalDeviceGuard;
pub use raii::surface_guard::SurfaceGuard;
pub use raii::swap_chain_guard::{query_swap_chain_support, SwapChainGuard};
use simple_logger::{set_up_color_terminal, SimpleLogger};

pub fn init_logging() -> Result<()> {
    set_up_color_terminal();
    let logger = SimpleLogger::new();
    logger.init()?;
    Ok(())
}
