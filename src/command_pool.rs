use std::rc::Rc;

use crate::{frame::Frame, GraphicsPipeline, LogicalDevice};

use anyhow::Result;
use ash::vk::{
    self, CommandBufferAllocateInfo, CommandBufferLevel, CommandPoolCreateFlags,
    CommandPoolCreateInfo,
};

pub struct CommandPool {
    frame_idx: usize,
    frames: Vec<Frame>,
    command_pool: vk::CommandPool,
    logical_device: Rc<LogicalDevice>,
}

const FRAMES_IN_FLIGHT: u32 = 1;

impl CommandPool {
    pub fn new(
        logical_device: &Rc<LogicalDevice>,
        graphics_pipeline: GraphicsPipeline,
    ) -> Result<Self> {
        let queue_family_indicies = logical_device.get_queue_family_indicies();

        let create_command_pool = CommandPoolCreateInfo::default()
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_indicies.graphics_family.unwrap() as u32);
        let command_pool =
            unsafe { logical_device.create_command_pool(&create_command_pool, None)? };

        let allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(FRAMES_IN_FLIGHT);

        let command_buffers = unsafe { logical_device.allocate_command_buffers(&allocate_info)? };
        let graphics_pipeline = Rc::new(graphics_pipeline);

        let frames = command_buffers
            .into_iter()
            .map(|command_buffer| Frame::new(logical_device, command_buffer, &graphics_pipeline))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            frame_idx: 0,
            frames,
            command_pool,
            logical_device: Rc::clone(logical_device),
        })
    }

    pub fn get_next_frame(&mut self) -> &Frame {
        let frame = &self.frames[self.frame_idx];
        self.frame_idx = (self.frame_idx + 1) % self.frames.len();
        frame
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
