mod command_pool_guard;

use std::rc::Rc;

use anyhow::Result;

use super::LogicalDeviceGuard;

pub use self::command_pool_guard::CommandPoolGuard;

pub fn create_command_pool(
    logical_device: &Rc<LogicalDeviceGuard>,
    queue_family_index: u32,
) -> Result<CommandPoolGuard> {
    CommandPoolGuard::try_new(logical_device, queue_family_index)
}
