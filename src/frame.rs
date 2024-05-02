use std::rc::Rc;

use ash::vk::{
    ClearColorValue, ClearValue, CommandBuffer, CommandBufferBeginInfo, CommandBufferResetFlags,
    Fence, FenceCreateFlags, FenceCreateInfo, PipelineBindPoint, PipelineStageFlags,
    PresentInfoKHR, Rect2D, RenderPassBeginInfo, Semaphore, SemaphoreCreateInfo, SubmitInfo,
    SubpassContents,
};

use anyhow::Result;

use crate::{GraphicsPipeline, LogicalDevice, Swapchain};

/// Struct representing an abstract "Frame" that can be
/// rendered. Contains the resources needed for a particular
/// frame rendering loop.
pub struct Frame {
    logical_device: Rc<LogicalDevice>,
    graphics_pipeline: Rc<GraphicsPipeline>,

    pub command_buffer: CommandBuffer,
    /// Semaphore for when the image is available to be used from the
    /// swapchain
    pub image_available_semaphore: Semaphore,
    /// Semaphore for when the rendering has finished
    pub render_finished_semaphore: Semaphore,
    /// Fence for synchronizing render passes
    pub in_flight_fence: Fence,
}

impl Frame {
    pub fn new(
        logical_device: &Rc<LogicalDevice>,
        command_buffer: CommandBuffer,
        graphics_pipeline: &Rc<GraphicsPipeline>,
    ) -> Result<Self> {
        let semaphore_create_info = SemaphoreCreateInfo::default();
        let fence_create_info = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);

        let image_available_semaphore =
            unsafe { logical_device.create_semaphore(&semaphore_create_info, None)? };
        let render_finished_semaphore =
            unsafe { logical_device.create_semaphore(&semaphore_create_info, None)? };
        let in_flight_fence = unsafe { logical_device.create_fence(&fence_create_info, None)? };

        Ok(Self {
            logical_device: Rc::clone(logical_device),
            command_buffer,
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
            graphics_pipeline: Rc::clone(graphics_pipeline),
        })
    }

    pub fn render(&self, swapchain: &Swapchain) -> Result<()> {
        let fences = [self.in_flight_fence];
        unsafe {
            // wait for previous draw to complete
            self.logical_device
                .wait_for_fences(&fences, true, u64::MAX)?;
            // reset the fence so that it can be re-signaled when this draw is complete
            self.logical_device.reset_fences(&fences)?;
        }

        let image_index = swapchain.acquire_next_image_index(&self.image_available_semaphore)?;

        unsafe {
            self.logical_device
                .reset_command_buffer(self.command_buffer, CommandBufferResetFlags::empty())?
        }

        self.record_command_buffer(image_index as usize, swapchain)?;

        let wait_semaphores = [self.image_available_semaphore];
        let signal_semaphores = [self.render_finished_semaphore];
        let pipeline_stage_flags = [PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [self.command_buffer];
        let submit_info = [SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&pipeline_stage_flags)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)];
        unsafe {
            self.logical_device.queue_submit(
                self.logical_device.get_queues().graphics,
                &submit_info,
                self.in_flight_fence,
            )?
        }

        let swapchains = [*swapchain.get_handle()];
        let image_indicies = [image_index];
        let present_info = PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indicies);
        unsafe {
            swapchain.queue_present(self.logical_device.get_queues().present, &present_info)?
        };

        Ok(())
    }

    /// Records the command buffer for execution
    fn record_command_buffer(&self, image_index: usize, swapchain: &Swapchain) -> Result<()> {
        let command_buffer_begin_info = CommandBufferBeginInfo::default();
        unsafe {
            self.logical_device
                .begin_command_buffer(self.command_buffer, &command_buffer_begin_info)?
        };

        let swapchain_extent = swapchain.get_extent();
        let render_area = Rect2D::default().extent(*swapchain_extent);

        let mut clear_value = ClearValue::default();
        clear_value.color = ClearColorValue {
            uint32: [0, 0, 0, 1],
        };
        let clear_values = [clear_value];

        let render_pass_begin_info = RenderPassBeginInfo::default()
            .render_pass(**self.graphics_pipeline.get_render_pass())
            .framebuffer(
                **self
                    .graphics_pipeline
                    .get_framebuffer_for_index(image_index),
            )
            .render_area(render_area)
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
                **self.graphics_pipeline,
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

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_fence(self.in_flight_fence, None);
            self.logical_device
                .destroy_semaphore(self.image_available_semaphore, None);
            self.logical_device
                .destroy_semaphore(self.render_finished_semaphore, None);
        }
    }
}
