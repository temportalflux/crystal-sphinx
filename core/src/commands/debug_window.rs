use super::CommandList;
use engine::{input, ui::egui::Element};

pub struct DebugWindow {
	is_open: bool,
	commands: CommandList,
}

impl DebugWindow {
	pub fn new(commands: CommandList) -> Self {
		Self {
			is_open: false,
			commands,
		}
	}
}

impl Element for DebugWindow {
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
