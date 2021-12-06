use super::AppStateView;
use engine::ui::{
	oui::{
		widget::{container::content_box, SizeBox},
		AsRAUI, Widget,
	},
	raui::*,
};

pub struct Loading {
	root: content_box::Container,
}

impl AppStateView for Loading {
	fn new() -> Self {
		let root =
			content_box::Container::new().with_slot(SizeBox::new().with_navicability().arclocked());
		Self { root }
	}
}

impl Widget for Loading {}

impl AsRAUI for Loading {
	fn as_raui(&self) -> WidgetComponent {
		self.root.as_raui()
	}
}
