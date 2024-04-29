use std::{ops::Deref, rc::Rc};

use anyhow::Result;
use ash::vk::{
    self, ComponentMapping, ComponentSwizzle, Image, ImageAspectFlags, ImageSubresourceRange,
    ImageViewCreateInfo, ImageViewType, SurfaceFormatKHR,
};

use crate::LogicalDevice;

pub struct ImageView {
    logical_device: Rc<LogicalDevice>,
    image_view: vk::ImageView,
    // need to keep references to these to make sure they aren't
    // cleaned up before we are
    _image: Image,
}

impl ImageView {
    pub fn new(
        logical_device: &Rc<LogicalDevice>,
        surface_format: SurfaceFormatKHR,
        image: Image,
    ) -> Result<Self> {
        let image_view_create_info = ImageViewCreateInfo::default()
            .image(image)
            // 2D images
            .view_type(ImageViewType::TYPE_2D)
            .format(surface_format.format)
            // no swizzling
            .components(
                ComponentMapping::default()
                    .a(ComponentSwizzle::IDENTITY)
                    .b(ComponentSwizzle::IDENTITY)
                    .g(ComponentSwizzle::IDENTITY)
                    .r(ComponentSwizzle::IDENTITY),
            )
            // color images with no mipmapping or layers
            .subresource_range(
                ImageSubresourceRange::default()
                    .aspect_mask(ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        let image_view =
            unsafe { logical_device.create_image_view(&image_view_create_info, None)? };

        Ok(Self {
            logical_device: Rc::clone(logical_device),
            image_view,
            _image: image,
        })
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            self.logical_device
                .destroy_image_view(self.image_view, None)
        }
    }
}

impl Deref for ImageView {
    type Target = vk::ImageView;

    fn deref(&self) -> &Self::Target {
        &self.image_view
    }
}
