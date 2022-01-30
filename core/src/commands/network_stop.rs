use super::Command;
use crate::app;
use crate::common::network::mode;
use std::sync::{Arc, RwLock};

pub struct UnloadNetwork {
	app_state: Arc<RwLock<app::state::Machine>>,
}

impl UnloadNetwork {
	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		Self { app_state }
	}

	fn is_dedicated_client(&self) -> bool {
		mode::get() == mode::Kind::Client
	}
}

impl Command for UnloadNetwork {
	fn is_allowed(&self) -> bool {
		let current_state = self.app_state.read().unwrap().get();
		current_state == app::state::State::InGame
	}

	fn render(&mut self, ui: &mut egui::Ui) {
		use app::state::State::*;
		let is_client = self.is_dedicated_client();
		let (label, next_state) = match is_client {
			true => ("Disconnect", Disconnecting),
			false => ("Unload World", Unloading),
		};
		if ui.button(label).clicked() {
			self.app_state
				.write()
				.unwrap()
				.transition_to(next_state, None);
		}
	}
}
