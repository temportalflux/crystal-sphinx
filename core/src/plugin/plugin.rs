use crate::app;

pub trait Plugin {
	fn name(&self) -> &'static str;
	fn version(&self) -> semver::Version;

	fn register_state_background(
		&self,
		state: app::state::State,
		list: &mut Vec<engine::asset::Id>,
	);
	// temporary proof of concept function, need to have game phases at some point
	fn register_main_menu_music(&self, _list: &mut engine::asset::WeightedIdList) {}
}

impl std::fmt::Display for dyn Plugin + 'static + Send + Sync {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}(v{})", self.name(), self.version())
	}
}

impl std::fmt::Debug for dyn Plugin + 'static + Send + Sync {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}(v{})", self.name(), self.version())
	}
}
