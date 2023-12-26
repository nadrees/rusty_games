#[cfg(debug_assertions)]
mod debug_utils_guard;

use anyhow::Result;

use ash::{Entry, Instance};

#[cfg(debug_assertions)]
pub use crate::vulkan::extensions_registry::debug_utils_guard::DebugUtilsGuard;

/// Public trait used as a marker interface so we can return multiple extensions generically
pub trait Extension {}

/// Internal trait all extensions are expected to implement
trait ExtensionImpl: Extension {
    fn name() -> String;
    fn try_new(entry: &Entry, instance: &Instance) -> Result<Self>
    where
        Self: Sized;
}

impl<T> Extension for T where T: ExtensionImpl {}

pub fn get_names() -> Vec<String> {
    #[cfg(debug_assertions)]
    {
        vec![DebugUtilsGuard::name()]
    }
    #[cfg(not(debug_assertions))]
    {
        vec![]
    }
}

pub fn create_extensions(entry: &Entry, instance: &Instance) -> Result<Vec<Box<dyn Extension>>> {
    #[cfg(debug_assertions)]
    {
        let debug_utils = DebugUtilsGuard::try_new(entry, instance)?;
        Ok(vec![Box::new(debug_utils)])
    }
    #[cfg(not(debug_assertions))]
    {
        Ok(vec![])
    }
}
