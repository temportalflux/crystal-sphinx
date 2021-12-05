use crate::app;
use engine::{input, ui::egui::Element};
use std::sync::{Arc, RwLock};

pub struct DebugCommands {
	is_open: bool,
	app_state: Arc<RwLock<app::state::Machine>>,
}

impl DebugCommands {
	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		Self {
			is_open: false,
			app_state,
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

		let app_state_ref = self.app_state.clone();
		let current_state = self.app_state.read().unwrap().get();
		egui::Window::new("Debug Commands")
			.open(&mut self.is_open)
			.show(ctx, move |ui| {
				if current_state == app::state::State::MainMenu {
					if ui.button("Load World").clicked() {
						app_state_ref
							.write()
							.unwrap()
							.transition_to(app::state::State::LoadingWorld);
					}
				}
			});
	}
}
