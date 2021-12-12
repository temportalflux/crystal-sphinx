use super::Command;
use crate::app;
use engine::network::{mode, LocalData};
use std::sync::{Arc, RwLock};

#[derive(PartialEq, Clone)]
pub enum WorldOption {
	New,
	Path(String),
}

impl WorldOption {
	fn to_transition_data(&self) -> app::state::TransitionData {
		use crate::network::task::{Directive, Instruction};
		let mode = mode::Set::all();
		let port = LocalData::get_named_arg("host_port");
		Some(Box::new(match self {
			Self::New => Instruction {
				mode,
				port,
				// TODO: Create a unique identifier based on a user-provided world name
				directive: Directive::LoadWorld("tmp".to_owned()),
			},
			Self::Path(path) => Instruction {
				mode,
				port,
				directive: Directive::LoadWorld(path.clone()),
			},
		}))
	}
}

impl std::fmt::Display for WorldOption {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::New => write!(f, "New World"),
			Self::Path(path) => write!(f, "{}", path.to_string()),
		}
	}
}

pub struct LoadNetwork {
	app_state: Arc<RwLock<app::state::Machine>>,
	selected_world: WorldOption,
	options: Vec<WorldOption>,
}

impl LoadNetwork {
	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		let options = vec![WorldOption::New]; // TODO: get the list of worlds from disk (saving a world isn't implemented yet)
		Self {
			app_state,
			selected_world: WorldOption::New,
			options,
		}
	}

	fn load_world(&self, world: &WorldOption) {
		self.app_state
			.write()
			.unwrap()
			.transition_to(app::state::State::LoadingWorld, world.to_transition_data());
	}
}

impl Command for LoadNetwork {
	fn is_allowed(&self) -> bool {
		let current_state = self.app_state.read().unwrap().get();
		current_state == app::state::State::MainMenu
	}

	fn render(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			egui::ComboBox::from_label("Select a world")
				.selected_text(&self.selected_world)
				.show_ui(ui, |ui| {
					for option in self.options.iter() {
						ui.selectable_value(&mut self.selected_world, option.clone(), option);
					}
				});
			if ui.button("Load World").clicked() {
				self.load_world(&self.selected_world);
			}
		});
	}
}
