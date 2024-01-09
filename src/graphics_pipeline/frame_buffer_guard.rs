use std::{ops::Deref, rc::Rc};

use anyhow::Result;
use ash::vk::{Framebuffer, FramebufferCreateInfo};
use tracing::debug;

use crate::logical_device::LogicalDeviceGuard;

use super::{
    image_view_guard::ImageViewGuard, render_pass_guard::RenderPassGuard,
    swap_chain_guard::SwapChainGuard,
};

pub struct FrameBufferGuard {
    _image_view: Rc<ImageViewGuard>,
    logical_device: Rc<LogicalDeviceGuard>,
    frame_buffer: Framebuffer,
    _render_pass: Rc<RenderPassGuard>,
}

impl FrameBufferGuard {
    pub fn try_new(
        image_view: &Rc<ImageViewGuard>,
        render_pass: &Rc<RenderPassGuard>,
        swap_chain: &SwapChainGuard,
        logical_device: &Rc<LogicalDeviceGuard>,
    ) -> Result<Self> {
        debug!("Creating frame buffer...");

        let attachments = [***image_view];
        let frame_buffer_create_info = FramebufferCreateInfo::builder()
            .render_pass(***render_pass)
            .attachments(&attachments)
            .width(swap_chain.extent.width)
            .height(swap_chain.extent.height)
            .layers(SwapChainGuard::LAYERS);
        let frame_buffer =
            unsafe { logical_device.create_framebuffer(&frame_buffer_create_info, None) }?;

        debug!("Frame buffer created");

        Ok(Self {
            _image_view: Rc::clone(image_view),
            logical_device: Rc::clone(logical_device),
            _render_pass: Rc::clone(render_pass),
            frame_buffer,
        })
    }
}

impl Drop for FrameBufferGuard {
    fn drop(&mut self) {
        debug!("Dropping FrameBufferGuard");
        unsafe {
            self.logical_device
                .destroy_framebuffer(self.frame_buffer, None)
        }
    }
}

impl Deref for FrameBufferGuard {
    type Target = Framebuffer;

    fn deref(&self) -> &Self::Target {
        &self.frame_buffer
    }
}
