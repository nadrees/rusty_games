mod command_pool;
mod fence_guard;
mod graphics_pipeline;
mod logical_device;
mod semaphore_guard;

use std::rc::Rc;

use anyhow::Result;
use ash::{vk::PipelineStageFlags, Entry};
use glfw::{Glfw, PWindow};

use self::{
    command_pool::{create_command_pool, CommandPoolGuard},
    fence_guard::FenceGuard,
    graphics_pipeline::{create_graphics_pipeline, GraphicsPipelineGuard},
    logical_device::{create_logical_device, LogicalDeviceGuard},
    semaphore_guard::SemaphoreGuard,
};

pub fn create_graphics_engine(glfw: &Glfw, window: &PWindow) -> Result<GraphcisEngine> {
    let entry = Entry::linked();
    let logical_device = create_logical_device(&entry, &glfw, &window)?;
    let graphics_pipeline = create_graphics_pipeline(&entry, &window, &logical_device)?;
    let graphics_command_pool =
        create_command_pool(&logical_device, logical_device.graphics_queue_family_index)?;
    let image_available_semaphore = SemaphoreGuard::try_new(&logical_device)?;
    let render_finished_semaphore = SemaphoreGuard::try_new(&logical_device)?;
    let frame_rendering_fence = FenceGuard::try_new(&logical_device, true)?;
    Ok(GraphcisEngine {
        frame_rendering_fence,
        graphics_pipeline,
        graphics_command_pool,
        image_available_semaphore,
        logical_device,
        render_finished_semaphore,
    })
}

pub struct GraphcisEngine {
    frame_rendering_fence: FenceGuard,
    graphics_pipeline: GraphicsPipelineGuard,
    graphics_command_pool: CommandPoolGuard,
    image_available_semaphore: SemaphoreGuard,
    logical_device: Rc<LogicalDeviceGuard>,
    render_finished_semaphore: SemaphoreGuard,
}

impl GraphcisEngine {
    pub fn render_frame(&self) -> Result<()> {
        self.wait_for_previous_frame_to_finish()?;

        let image_index = self.graphics_pipeline.swap_chain.get_next_image_index(
            u64::MAX,
            Some(&self.image_available_semaphore),
            None,
        )?;
        self.graphics_command_pool
            .record_command_buffer(&self.graphics_pipeline, image_index)?;
        self.graphics_command_pool.submit_command_buffer(
            self.logical_device.get_graphics_queue(),
            vec![&self.render_finished_semaphore],
            &self.frame_rendering_fence,
            vec![(&self.image_available_semaphore, PipelineStageFlags::empty())],
        )?;

        Ok(())
    }

    fn wait_for_previous_frame_to_finish(&self) -> Result<()> {
        let wait_for_fences = [*self.frame_rendering_fence];
        unsafe {
            self.logical_device
                .wait_for_fences(&wait_for_fences, true, u64::MAX)?;
            self.logical_device.reset_fences(&wait_for_fences)?;
        }
        Ok(())
    }
}
