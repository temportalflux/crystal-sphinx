pub trait Plugin {
	fn name(&self) -> &'static str;
	fn version(&self) -> semver::Version;
}

impl std::fmt::Display for dyn Plugin {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}(v{})", self.name(), self.version())
	}
}

impl std::fmt::Debug for dyn Plugin {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}(v{})", self.name(), self.version())
	}
}

#[derive(Default, Debug)]
pub struct Config {
	plugins: Vec<Box<dyn Plugin>>,
}

impl Config {
	pub fn with<T>(mut self, plugin: Box<T>) -> Self
	where
		T: Plugin + 'static,
	{
		self.plugins.push(plugin);
		self
	}
}
