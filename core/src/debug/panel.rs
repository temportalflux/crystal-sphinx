use engine::{input, ui::egui::Element};
use std::{cell::RefCell, rc::Rc};

/// The main view panel for the debug tools in-game.
pub struct Panel {
	is_open: bool,
	weak_action: input::action::WeakLockState,
	windows: Vec<(String, Rc<RefCell<dyn PanelWindow>>)>,
}

pub trait PanelWindow: Element {
	fn is_open_mut(&mut self) -> &mut bool;
}

impl Panel {
	pub fn new(arc_user: &input::ArcLockUser) -> Self {
		let weak_action =
			input::User::get_action_in(&arc_user, crate::input::ACTION_TOGGLE_DEBUG_CMDS).unwrap();
		Self {
			is_open: false,
			weak_action,
			windows: Vec::new(),
		}
	}

	pub fn with_window(mut self, id: impl ToString, window: impl PanelWindow + 'static) -> Self {
		self.windows
			.push((id.to_string(), Rc::new(RefCell::new(window))));
		self
	}
}

impl Element for Panel {
	fn render(&mut self, ctx: &egui::CtxRef) {
		if let Some(arc_state) = self.weak_action.upgrade() {
			let action = arc_state.read().unwrap();
			if action.on_button_pressed() {
				self.is_open = !self.is_open;
			}
		}
		if !self.is_open {
			return;
		}

		egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
			egui::menu::bar(ui, |ui| {
				egui::menu::menu(ui, "Windows", |ui| {
					for (id, window) in self.windows.iter() {
						let _ = ui.checkbox(window.borrow_mut().is_open_mut(), &id);
					}
				});
			});
		});

		for (_id, window) in self.windows.iter() {
			window.borrow_mut().render(ctx);
		}
	}
}
