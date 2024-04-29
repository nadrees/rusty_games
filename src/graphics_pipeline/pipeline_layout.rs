use std::{ops::Deref, rc::Rc};

use crate::LogicalDevice;
use anyhow::Result;
use ash::vk::{self, PipelineLayoutCreateInfo};

pub struct PipelineLayout {
    logical_device: Rc<LogicalDevice>,
    layout: vk::PipelineLayout,
}

impl PipelineLayout {
    pub fn new(logical_device: &Rc<LogicalDevice>) -> Result<Self> {
        let pipeline_layout_create_info = PipelineLayoutCreateInfo::default();
        let pipeline_layout =
            unsafe { logical_device.create_pipeline_layout(&pipeline_layout_create_info, None)? };

        Ok(Self {
            logical_device: Rc::clone(logical_device),
            layout: pipeline_layout,
        })
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_pipeline_layout(self.layout, None)
        }
    }
}

impl Deref for PipelineLayout {
    type Target = vk::PipelineLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}
