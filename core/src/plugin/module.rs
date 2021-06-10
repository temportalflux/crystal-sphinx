use crate::{
	engine::utility::AnyError,
	plugin::{Plugin, LOG},
};
use std::path::Path;

pub type FnInit = unsafe extern "C" fn() -> Box<dyn Plugin>;
pub static INITIALIZE_FUNC_NAME: &'static str = "initialize_plugin";

pub(crate) struct Module {
	// NOTE: if this needs to be an arc, just convert the plugin-provided box into an arc
	// https://doc.rust-lang.org/std/sync/struct.Arc.html#example-4
	pub plugin: Box<dyn Plugin>,
	_library: libloading::Library,
}

impl Module {
	pub(crate) fn load(library_file: &Path) -> Result<Self, AnyError> {
		let module = unsafe {
			let library = libloading::Library::new(library_file)?;
			let plugin = {
				let plugin_init: libloading::Symbol<FnInit> =
					library.get(INITIALIZE_FUNC_NAME.as_bytes())?;
				plugin_init()
			};
			Self {
				_library: library,
				plugin,
			}
		};
		log::info!(target: LOG, "Loaded plugin \"{}\"", module.plugin.name());
		Ok(module)
	}
}
