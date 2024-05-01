use std::{ops::Deref, rc::Rc};

use crate::{ImageView, LogicalDevice};

use anyhow::Result;
use ash::vk::{self, Extent2D, FramebufferCreateInfo};

use super::render_pass::RenderPass;

pub struct Framebuffer {
    logical_device: Rc<LogicalDevice>,
    framebuffer: vk::Framebuffer,
    // variables we need to hold onto so they dont get cleaned
    // up before we do
    _render_pass: Rc<RenderPass>,
    _image_view: ImageView,
}

impl Framebuffer {
    pub fn new(
        logical_device: &Rc<LogicalDevice>,
        render_pass: &Rc<RenderPass>,
        extent: &Extent2D,
        image_view: ImageView,
    ) -> Result<Self> {
        let attachments = [*image_view];
        let create_info = FramebufferCreateInfo::default()
            .render_pass(***render_pass)
            .attachments(&attachments)
            .height(extent.height)
            .width(extent.width)
            .layers(1);
        let framebuffer = unsafe { logical_device.create_framebuffer(&create_info, None)? };

        Ok(Self {
            framebuffer,
            logical_device: Rc::clone(logical_device),
            _image_view: image_view,
            _render_pass: Rc::clone(render_pass),
        })
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_framebuffer(self.framebuffer, None)
        }
    }
}

impl Deref for Framebuffer {
    type Target = vk::Framebuffer;

    fn deref(&self) -> &Self::Target {
        &self.framebuffer
    }
}
