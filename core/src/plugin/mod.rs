//! Module for loading and managing plugin code that gets loaded at runtime.
//! All plugins must defined an initialize function and a struct which implements `Plugin` in order to be loaded.
//!
//! ```
//! #[no_mangle]
//! pub extern "C" fn initialize_plugin() -> Box<dyn Plugin> {
//! 	Box::new(MyPlugin())
//! }
//!
//! struct MyPlugin();
//!
//! impl Plugin for MyPlugin {
//! 	fn name(&self) -> &'static str {
//! 		"myy-plugin"
//! 	}
//! }
//! ```
//!

pub(crate) static LOG: &'static str = "plugin";

mod manager;
pub use manager::*;
mod module;
pub use module::*;
mod plugin;
pub use plugin::*;
