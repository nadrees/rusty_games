use std::{ops::Deref, rc::Rc};

use anyhow::Result;
use ash::vk::{
    Image, ImageAspectFlags, ImageSubresourceRange, ImageView, ImageViewCreateInfo, ImageViewType,
    SurfaceFormatKHR,
};
use tracing::debug;

use crate::LogicalDeviceGuard;

pub struct ImageViewGuard {
    _image: Image,
    view: ImageView,
    logical_device: Rc<LogicalDeviceGuard>,
}

impl ImageViewGuard {
    pub fn try_new(
        image: Image,
        logical_device: &Rc<LogicalDeviceGuard>,
        surface_format: &SurfaceFormatKHR,
    ) -> Result<Rc<Self>> {
        let image_view_create_info = ImageViewCreateInfo::builder()
            .image(image)
            .view_type(ImageViewType::TYPE_2D)
            .format(surface_format.format)
            .subresource_range(
                ImageSubresourceRange::builder()
                    .aspect_mask(ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            );
        let image_view =
            unsafe { logical_device.create_image_view(&image_view_create_info, None) }?;
        Ok(Rc::new(Self {
            _image: image,
            logical_device: Rc::clone(logical_device),
            view: image_view,
        }))
    }
}

impl Drop for ImageViewGuard {
    fn drop(&mut self) {
        debug!("Dropping ImageViewGuard");
        unsafe { self.logical_device.destroy_image_view(self.view, None) }
    }
}

impl Deref for ImageViewGuard {
    type Target = ImageView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}
