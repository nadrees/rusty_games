use std::{collections::HashSet, ffi::CString, rc::Rc};

use anyhow::Result;
use ash::vk::{
    ColorSpaceKHR, Extent2D, Format, PhysicalDevice, PresentModeKHR, QueueFlags,
    SurfaceCapabilitiesKHR, SurfaceFormatKHR,
};
use winit::window::Window;

use crate::{Instance, Surface, REQUIRED_DEVICE_EXTENSIONS};

/// Struct representing the intersection of a physical device and
/// presentation surface. There should be one per surface to display
/// results on, and per physical device.
pub struct PhysicalDeviceSurface {
    instance: Rc<Instance>,
    surface: Rc<Surface>,
    physical_device: PhysicalDevice,
    queue_families: QueueFamilyIndicies,
    swapchain_support_details: SwapChainSupportDetails,
}

impl PhysicalDeviceSurface {
    pub fn new(
        instance: &Rc<Instance>,
        surface: &Rc<Surface>,
        physical_device: PhysicalDevice,
    ) -> Result<Self> {
        let queue_families = find_queue_families(instance, &physical_device, &surface);
        let swapchain_support_details = query_swap_chain_support(&physical_device, surface)?;
        Ok(Self {
            instance: Rc::clone(instance),
            surface: Rc::clone(surface),
            physical_device,
            queue_families,
            swapchain_support_details,
        })
    }

    pub fn get_surface_capabilities(&self) -> Result<SurfaceCapabilitiesKHR> {
        self.surface
            .get_physical_device_surface_capabilities(&self.physical_device)
    }

    pub fn get_surface_formats(&self) -> Result<Vec<SurfaceFormatKHR>> {
        self.surface
            .get_physical_device_surface_formats(&self.physical_device)
    }

    pub fn get_surface_present_modes(&self) -> Result<Vec<PresentModeKHR>> {
        self.surface
            .get_physical_device_surface_present_modes(&self.physical_device)
    }

    pub fn is_suitable(&self) -> Result<bool> {
        let supports_extensions = self.check_device_extensions_supported()?;
        let mut swap_chain_supported = false;
        if supports_extensions {
            swap_chain_supported = !self.swapchain_support_details.formats.is_empty()
                && !self.swapchain_support_details.present_modes.is_empty();
        }

        Ok(self.queue_families.is_complete() && supports_extensions && swap_chain_supported)
    }

    pub fn get_queue_family_indicies(&self) -> &QueueFamilyIndicies {
        &self.queue_families
    }

    pub fn get_physical_device(&self) -> PhysicalDevice {
        self.physical_device
    }

    pub fn get_swapchain_support_details(&self) -> &SwapChainSupportDetails {
        &self.swapchain_support_details
    }

    pub fn get_surface(&self) -> &Rc<Surface> {
        &self.surface
    }

    /// Checks to see if the physical device supports all required device extensions
    fn check_device_extensions_supported(&self) -> Result<bool> {
        let device_extension_properties = unsafe {
            self.instance
                .enumerate_device_extension_properties(self.physical_device)?
        };

        let mut device_extension_names = HashSet::new();
        for device_extension in device_extension_properties {
            let extension_name = device_extension.extension_name_as_c_str()?;
            device_extension_names.insert(extension_name.to_owned());
        }

        for required_extension in REQUIRED_DEVICE_EXTENSIONS {
            let required_extension_name: CString = (*required_extension).to_owned();
            if !device_extension_names.contains(&required_extension_name) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Queries the Queue Families the physica device supports, and records the index of the relevant ones.
fn find_queue_families(
    instance: &Instance,
    physical_device: &PhysicalDevice,
    surface: &Rc<Surface>,
) -> QueueFamilyIndicies {
    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };
    QueueFamilyIndicies {
        graphics_family: queue_family_properties
            .iter()
            .position(|qfp| qfp.queue_flags.contains(QueueFlags::GRAPHICS)),
        present_family: queue_family_properties
            .iter()
            .enumerate()
            .position(|(idx, _)| {
                surface
                    .get_physical_device_surface_support(physical_device, idx as u32)
                    .unwrap_or_default()
            }),
    }
}

/// Queries for the details of what the swap chain supports given
/// the physical device and surface
fn query_swap_chain_support(
    physical_device: &PhysicalDevice,
    surface: &Surface,
) -> Result<SwapChainSupportDetails> {
    let capabilities = surface.get_physical_device_surface_capabilities(physical_device)?;
    let formats = surface.get_physical_device_surface_formats(physical_device)?;
    let present_modes = surface.get_physical_device_surface_present_modes(physical_device)?;

    Ok(SwapChainSupportDetails {
        capabilities,
        formats,
        present_modes,
    })
}

pub struct QueueFamilyIndicies {
    /// The graphics queue family index, if one is available
    pub graphics_family: Option<usize>,
    pub present_family: Option<usize>,
}

impl QueueFamilyIndicies {
    /// True if all queue families are available for this physical
    /// device.
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

#[derive(Clone)]
/// Details about what features the swap chain supports
/// for a given surface
pub struct SwapChainSupportDetails {
    pub capabilities: SurfaceCapabilitiesKHR,
    /// The formats (color depth settings) available to use.
    formats: Vec<SurfaceFormatKHR>,
    present_modes: Vec<PresentModeKHR>,
}

impl SwapChainSupportDetails {
    /// Picks the preferential surface format to use from the available
    pub fn choose_swap_surface_format(&self) -> &SurfaceFormatKHR {
        let srgb_color_space_formats = self
            .formats
            .iter()
            .filter(|format| format.color_space == ColorSpaceKHR::SRGB_NONLINEAR)
            .collect::<Vec<_>>();
        if let Some(b8g8r8a8_format) = srgb_color_space_formats
            .iter()
            .find(|format| format.format == Format::B8G8R8A8_SRGB)
        {
            return *b8g8r8a8_format;
        } else if let Some(srbg_format) = srgb_color_space_formats.first() {
            return *srbg_format;
        } else {
            return self.formats.first().unwrap();
        }
    }

    /// Picks the preferential swap mode to use based on the available
    pub fn choose_swap_present_mode(&self) -> PresentModeKHR {
        // prefer mailbox, where if we can render faster than the screen can present
        // and the queue fills up, we'll replace the last image with the most up to
        // date version
        if self.present_modes.contains(&PresentModeKHR::MAILBOX) {
            return PresentModeKHR::MAILBOX;
        }
        // otherwise, use FIFO - basically vertical sync. This is the only setting
        // guaranteed to be available on all systems
        return PresentModeKHR::FIFO;
    }

    /// Returns the "extent" of the images to draw - the resolution to use *in pixels*.
    pub fn choose_swap_extent(&self, window: &Window) -> Extent2D {
        match self.capabilities.current_extent.width {
            // in this scenario, we're in a high DPI setting where extent is in screen
            // space, but we need it to be in pixels. set it to the same size as the
            // window
            u32::MAX => {
                let window_size = window.inner_size();
                Extent2D {
                    width: window_size.width.clamp(
                        self.capabilities.min_image_extent.width,
                        self.capabilities.max_image_extent.width,
                    ),
                    height: window_size.height.clamp(
                        self.capabilities.min_image_extent.height,
                        self.capabilities.max_image_extent.height,
                    ),
                }
            }
            _ => self.capabilities.current_extent,
        }
    }

    /// Returns how many images the swap chain should use based on its support
    pub fn get_image_count(&self) -> u32 {
        let max_image_count = self.capabilities.max_image_count;
        let min_image_count = self.capabilities.min_image_count;

        let image_count = match max_image_count {
            // zero means there is no max, so just use 1 more than the minimum
            0 => min_image_count + 1,
            // in this case, use a few more than the minimum so we're not
            // stuck waiting on internal details
            _ => {
                let delta = (max_image_count - min_image_count) / 2;
                min_image_count + delta
            }
        };

        image_count.clamp(min_image_count, max_image_count)
    }
}
