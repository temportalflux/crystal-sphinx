use super::Plugin;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct Config {
	pub(super) plugins: Vec<Arc<dyn Plugin>>,
}

impl Config {
	pub fn with<T>(mut self, plugin: T) -> Self
	where
		T: Plugin + 'static,
	{
		self.plugins.push(Arc::new(plugin));
		self
	}
}
