use engine::ui::{
	oui::{
		widget::{container::content_box, SizeBox},
		AsRAUI, Widget,
	},
	raui::*,
};

pub struct Hud {
	root: content_box::Container,
}

impl Hud {
	pub fn new() -> Self {
		let root =
			content_box::Container::new().with_slot(SizeBox::new().with_navicability().arclocked());
		Self { root }
	}
}

impl Widget for Hud {}

impl AsRAUI for Hud {
	fn as_raui(&self) -> WidgetComponent {
		self.root.as_raui()
	}
}
