use std::rc::Rc;

use crate::LogicalDevice;

use anyhow::Result;
use ash::vk::{
    self, CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPoolCreateFlags,
    CommandPoolCreateInfo,
};

pub struct CommandPool {
    command_buffer: CommandBuffer,
    command_pool: vk::CommandPool,
    logical_device: Rc<LogicalDevice>,
}

impl CommandPool {
    pub fn new(logical_device: &Rc<LogicalDevice>) -> Result<Self> {
        let queue_family_indicies = logical_device.get_queue_family_indicies();

        let create_command_pool = CommandPoolCreateInfo::default()
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_indicies.graphics_family.unwrap() as u32);
        let command_pool =
            unsafe { logical_device.create_command_pool(&create_command_pool, None)? };

        let allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = unsafe { logical_device.allocate_command_buffers(&allocate_info)? };

        Ok(Self {
            command_buffer: command_buffers[0],
            command_pool,
            logical_device: Rc::clone(logical_device),
        })
    }

    pub fn get_command_buffer(&self) -> &CommandBuffer {
        &self.command_buffer
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_command_pool(self.command_pool, None)
        }
    }
}
