use crate::commands::CommandList;
use engine::ui::egui::Element;

pub struct CommandWindow {
	is_open: bool,
	commands: CommandList,
}

impl CommandWindow {
	pub fn new(commands: CommandList) -> Self {
		Self {
			is_open: false,
			commands,
		}
	}
}

impl super::PanelWindow for CommandWindow {
	fn is_open_mut(&mut self) -> &mut bool {
		&mut self.is_open
	}
}

impl Element for CommandWindow {
	fn render(&mut self, ctx: &egui::CtxRef) {
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
