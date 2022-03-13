use super::Plugin;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct Config {
	pub(super) plugins: Vec<Arc<dyn Plugin + 'static + Send + Sync>>,
}

impl Config {
	pub fn with<T>(mut self, plugin: T) -> Self
	where
		T: Plugin + 'static + Send + Sync,
	{
		self.plugins.push(Arc::new(plugin));
		self
	}
}
