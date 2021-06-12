pub trait Plugin {
	fn name(&self) -> &'static str;
	fn version(&self) -> semver::Version;

	// temporary proof of concept function, need to have game phases at some point
	fn register_main_menu_music(&self, _list: &mut crate::engine::asset::WeightedIdList) {}
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
