use std::rc::Rc;

use anyhow::Result;
use ash::vk::{
    ClearColorValue, ClearValue, CommandBuffer, CommandBufferAllocateInfo, CommandBufferBeginInfo,
    CommandBufferLevel, CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo, Offset2D,
    PipelineBindPoint, Rect2D, RenderPassBeginInfo, SubpassContents,
};
use tracing::debug;

use crate::{graphics_pipeline::GraphicsPipelineGuard, logical_device::LogicalDeviceGuard};

pub struct CommandPoolGuard {
    command_buffer: CommandBuffer,
    command_pool: CommandPool,
    logical_device: Rc<LogicalDeviceGuard>,
}

impl CommandPoolGuard {
    pub fn try_new(
        logical_device: &Rc<LogicalDeviceGuard>,
        queue_family_index: u32,
    ) -> Result<Self> {
        debug!("Creating command pool...");

        let command_pool_create_info = CommandPoolCreateInfo::builder()
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);
        let command_pool =
            unsafe { logical_device.create_command_pool(&command_pool_create_info, None) }?;

        let command_buffer_allocate_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let command_buffers =
            unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info) }?;

        debug!("Command pool created");

        Ok(Self {
            command_buffer: command_buffers[0],
            command_pool,
            logical_device: Rc::clone(logical_device),
        })
    }

    pub fn record_command_buffer(
        &self,
        graphics_pipeline: &GraphicsPipelineGuard,
        frame_index: usize,
    ) -> Result<()> {
        let command_buffer_begin_info = CommandBufferBeginInfo::builder();
        unsafe {
            self.logical_device
                .begin_command_buffer(self.command_buffer, &command_buffer_begin_info)
        }?;

        let clear_values = [clear_color()];
        let render_pass_begin_info = RenderPassBeginInfo::builder()
            .render_pass(**graphics_pipeline.render_pass)
            .framebuffer(*graphics_pipeline.frame_buffers[frame_index])
            .render_area(
                Rect2D::builder()
                    .offset(Offset2D::default())
                    .extent(graphics_pipeline.swap_chain.extent)
                    .build(),
            )
            .clear_values(&clear_values);
        unsafe {
            self.logical_device.cmd_begin_render_pass(
                self.command_buffer,
                &render_pass_begin_info,
                SubpassContents::INLINE,
            );
            self.logical_device.cmd_bind_pipeline(
                self.command_buffer,
                PipelineBindPoint::GRAPHICS,
                **graphics_pipeline,
            );
            self.logical_device
                .cmd_draw(self.command_buffer, 3, 1, 0, 0);
            self.logical_device.cmd_end_render_pass(self.command_buffer);
            self.logical_device
                .end_command_buffer(self.command_buffer)?;
        };

        Ok(())
    }
}

impl Drop for CommandPoolGuard {
    fn drop(&mut self) {
        debug!("Dropping CommandPoolGuard");
        unsafe {
            self.logical_device
                .destroy_command_pool(self.command_pool, None)
        }
    }
}

#[inline]
const fn clear_color() -> ClearValue {
    ClearValue {
        color: ClearColorValue {
            uint32: [0, 0, 0, 1],
        },
    }
}
