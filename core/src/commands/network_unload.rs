use super::Command;
use crate::app;
use std::sync::{Arc, RwLock};

pub struct UnloadNetwork {
	app_state: Arc<RwLock<app::state::Machine>>,
}

impl UnloadNetwork {
	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		Self { app_state }
	}
}

impl Command for UnloadNetwork {
	fn is_allowed(&self) -> bool {
		let current_state = self.app_state.read().unwrap().get();
		current_state == app::state::State::InGame
	}

	fn render(&mut self, ui: &mut egui::Ui) {
		if ui.button("Unload World").clicked() {
			self.app_state
				.write()
				.unwrap()
				.transition_to(app::state::State::Unloading, None);
			let _ = engine::network::Network::stop();
		}
	}
}
