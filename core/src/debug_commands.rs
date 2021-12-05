use crate::app;
use engine::{input, ui::egui::Element};
use std::sync::{Arc, RwLock, Mutex};

trait Command {
	fn is_allowed(&self) -> bool;
	fn render(&mut self, ui: &mut egui::Ui);
}

type CommandList = Vec<Arc<Mutex<dyn Command + 'static>>>;

pub struct DebugCommands {
	is_open: bool,
	commands: CommandList,
}

impl DebugCommands {
	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		Self {
			is_open: false,
			commands: vec![
				Arc::new(Mutex::new(LoadWorldCommand::new(app_state)))
			],
		}
	}
}

impl Element for DebugCommands {
	fn render(&mut self, ctx: &egui::CtxRef) {
		if let Some(action) =
			input::read().get_user_action(0, crate::input::ACTION_TOGGLE_DEBUG_CMDS)
		{
			if !self.is_open && action.on_button_pressed() {
				self.is_open = true;
			}
		}
		if !self.is_open {
			return;
		}

		let cmds = self.commands.clone();
		egui::Window::new("Debug Commands")
			.open(&mut self.is_open)
			.show(ctx, move |ui| {
				for arc_cmd in cmds.iter() {
					let mut command = arc_cmd.lock().unwrap();
					if command.is_allowed() {
						command.render(ui);
					}
				}
			});
	}
}

#[derive(PartialEq, Clone)]
enum WorldOption {
	New,
	Path(String),
}

impl std::fmt::Display for WorldOption {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::New => write!(f, "New World"),
			Self::Path(path) => write!(f, "{}", path.to_string())
		}
	}
}

struct LoadWorldCommand {
	app_state: Arc<RwLock<app::state::Machine>>,
	selected_world: WorldOption,
	options: Vec<WorldOption>,
}

impl LoadWorldCommand {
	fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		let options = vec![WorldOption::New]; // TODO: get the list of worlds from disk (saving a world isn't implemented yet)
		Self {
			app_state,
			selected_world: WorldOption::New,
			options,
		}
	}

	fn load_world(&self, world: &WorldOption) {
		log::debug!("Load \"{}\"", world);
		self.app_state
			.write()
			.unwrap()
			.transition_to(app::state::State::LoadingWorld);
	}

}

impl Command for LoadWorldCommand {
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
