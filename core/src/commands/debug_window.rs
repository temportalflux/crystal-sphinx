use super::CommandList;
use engine::{input, ui::egui::Element};

pub struct DebugWindow {
	is_open: bool,
	commands: CommandList,
	weak_action: input::action::WeakLockState,
}

impl DebugWindow {
	pub fn new(commands: CommandList, arc_user: &input::ArcLockUser) -> Self {
		let weak_action =
			input::User::get_action_in(&arc_user, crate::input::ACTION_TOGGLE_DEBUG_CMDS).unwrap();
		Self {
			is_open: false,
			commands,
			weak_action,
		}
	}
}

impl Element for DebugWindow {
	fn render(&mut self, ctx: &egui::CtxRef) {
		if let Some(arc_state) = self.weak_action.upgrade() {
			let action = arc_state.read().unwrap();
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
				let command_list = cmds.lock().unwrap();
				for arc_cmd in command_list.iter() {
					let mut command = arc_cmd.lock().unwrap();
					if command.is_allowed() {
						command.render(ui);
					}
				}
			});
	}
}
