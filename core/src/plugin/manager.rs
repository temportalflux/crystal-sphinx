use super::{Config, Plugin, LOG};
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Default)]
pub struct Manager {
	plugins: Vec<Arc<dyn Plugin>>,
}

impl Manager {
	fn get() -> &'static RwLock<Self> {
		use temportal_engine::utility::singleton::*;
		static mut INSTANCE: Singleton<Manager> = Singleton::uninit();
		unsafe { INSTANCE.get_or_default() }
	}

	pub fn write() -> LockResult<RwLockWriteGuard<'static, Self>> {
		Self::get().write()
	}

	pub fn read() -> LockResult<RwLockReadGuard<'static, Self>> {
		Self::get().read()
	}
}

impl Manager {
	pub fn load(&mut self, config: Config) {
		for plugin_arc in config.plugins.iter() {
			log::info!(target: LOG, "Using plugin {}", plugin_arc);
			self.plugins.push(plugin_arc.clone());
		}
	}

	pub fn register_main_menu_music(&self, list: &mut crate::engine::asset::WeightedIdList) {
		for plugin in self.plugins.iter() {
			plugin.register_main_menu_music(list);
		}
	}
}
