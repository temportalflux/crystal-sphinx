use super::Command;
use crate::common::network::mode;
use crate::common::utility::get_named_arg;
use crate::{app, common::network::task::Instruction};
use std::sync::{Arc, RwLock};

pub struct Connect {
	app_state: Arc<RwLock<app::state::Machine>>,
	url: String,
}

impl Connect {
	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		Self {
			app_state,
			url: "127.0.0.1:25565".to_string(),
		}
	}
}

impl Command for Connect {
	fn is_allowed(&self) -> bool {
		let current_state = self.app_state.read().unwrap().get();
		current_state == app::state::State::MainMenu
	}

	fn render(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			ui.text_edit_singleline(&mut self.url);
			if ui.button("Connect").clicked() {
				self.app_state.write().unwrap().transition_to(
					app::state::State::Connecting,
					Some(Box::new(Instruction {
						mode: mode::Kind::Client.into(),
						port: get_named_arg("client_port"),
						world_name: None,
						server_url: Some(self.url.clone()),
					})),
				);
			}
		});
	}
}
