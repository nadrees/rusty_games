use std::{ops::Deref, rc::Rc};

use anyhow::Result;
use ash::vk::{
    AttachmentDescription, AttachmentLoadOp, AttachmentReference, AttachmentStoreOp, ImageLayout,
    PipelineBindPoint, RenderPass, RenderPassCreateInfo, SampleCountFlags, SubpassDescription,
};
use tracing::debug;

use super::{swap_chain_guard::SwapChainGuard, LogicalDeviceGuard};

pub struct RenderPassGuard {
    render_pass: RenderPass,
    logical_device: Rc<LogicalDeviceGuard>,
}

impl RenderPassGuard {
    pub fn try_new(
        logical_device: &Rc<LogicalDeviceGuard>,
        swapchain: &SwapChainGuard,
    ) -> Result<Rc<Self>> {
        debug!("Creating render pass...");

        let attachment_descriptions = [AttachmentDescription::builder()
            .format(swapchain.surface_format.format)
            // no multi-sampling, so only need 1 sample
            .samples(SampleCountFlags::TYPE_1)
            // clear buffer before each draw
            .load_op(AttachmentLoadOp::CLEAR)
            // store the results back in the buffer so we can see it
            .store_op(AttachmentStoreOp::STORE)
            // not using stenciling yet, so ignore this
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            // dont care about previous image format before starting render pass
            .initial_layout(ImageLayout::UNDEFINED)
            // want to present final image to swap chain
            .final_layout(ImageLayout::PRESENT_SRC_KHR)
            .build()];

        // configure sub-passes (ex: for post-processing effects)
        // not doing any post-processing right now, so only need 1 subpass
        let attachment_ref = AttachmentReference::builder()
            // zero, because we only have 1 attachment description
            .attachment(0)
            // this reference is a color buffer
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();
        // this is a graphics subpass for displaying images from the color buffer
        let subpasses = [SubpassDescription::builder()
            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
            .color_attachments(&[attachment_ref])
            .build()];

        // create render pass
        let render_pass_create_info = RenderPassCreateInfo::builder()
            .attachments(&attachment_descriptions)
            .subpasses(&subpasses);
        let render_pass =
            unsafe { logical_device.create_render_pass(&render_pass_create_info, None) }?;

        debug!("Render pass created");

        Ok(Rc::new(Self {
            render_pass,
            logical_device: Rc::clone(logical_device),
        }))
    }
}

impl Drop for RenderPassGuard {
    fn drop(&mut self) {
        debug!("Dropping RenderPassGuard");
        unsafe {
            self.logical_device
                .destroy_render_pass(self.render_pass, None)
        }
    }
}

impl Deref for RenderPassGuard {
    type Target = RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}
