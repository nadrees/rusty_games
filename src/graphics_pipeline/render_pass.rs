use std::{ops::Deref, rc::Rc};

use crate::{LogicalDevice, Swapchain};

use anyhow::Result;
use ash::vk::{
    self, AccessFlags, AttachmentDescription, AttachmentLoadOp, AttachmentReference,
    AttachmentStoreOp, ImageLayout, PipelineBindPoint, PipelineStageFlags, RenderPassCreateInfo,
    SampleCountFlags, SubpassDependency, SubpassDescription, SUBPASS_EXTERNAL,
};

pub struct RenderPass {
    logical_device: Rc<LogicalDevice>,
    render_pass: vk::RenderPass,
}

impl RenderPass {
    pub fn new(logical_device: &Rc<LogicalDevice>, swapchain: &Swapchain) -> Result<Self> {
        let attachment_description = [AttachmentDescription::default()
            // ensure attachment format matches that of swapchain
            .format(swapchain.get_surface_format().format)
            // not using multisampling, so stick to 1 sample
            .samples(SampleCountFlags::TYPE_1)
            // clear the data in the attachment before rendering
            .load_op(AttachmentLoadOp::CLEAR)
            // dont care about layout of previous image, because we're clearing it
            // anyway
            .initial_layout(ImageLayout::UNDEFINED)
            // store the results in memory for later user after rendering
            .store_op(AttachmentStoreOp::STORE)
            // transition to a layout suitable for presentation
            .final_layout(ImageLayout::PRESENT_SRC_KHR)
            // not using stencils
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)];

        let attachment_ref = [AttachmentReference::default()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

        let subpass_description = [SubpassDescription::default()
            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
            .color_attachments(&attachment_ref)];

        let subpass_dependencies = [SubpassDependency::default()
            .src_subpass(SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags::empty())
            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)];

        let render_pass_create_info = RenderPassCreateInfo::default()
            .attachments(&attachment_description)
            .subpasses(&subpass_description)
            .dependencies(&subpass_dependencies);

        let render_pass =
            unsafe { logical_device.create_render_pass(&render_pass_create_info, None)? };

        Ok(Self {
            logical_device: Rc::clone(logical_device),
            render_pass,
        })
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_render_pass(self.render_pass, None)
        }
    }
}

impl Deref for RenderPass {
    type Target = vk::RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}
