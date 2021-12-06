use engine::ui::{
	oui::{
		widget::{container::content_box, ImageBox, SizeBox},
		AsRAUI, Widget,
	},
	raui::*,
};

pub struct Loading {
	root: content_box::Container,
	backgrounds: Vec<engine::asset::Id>,
}

impl Loading {
	pub fn new() -> Self {
		let mut backgrounds = Vec::new();
		if let Ok(manager) = crate::plugin::Manager::read() {
			manager.register_state_background(
				crate::app::state::State::LoadingWorld,
				&mut backgrounds,
			);
		}

		let mut root = content_box::Container::new();
		if !backgrounds.is_empty() {
			root = root.with_slot(content_box::Slot::from(
				ImageBox::new()
					.with_width(ImageBoxSizeValue::Fill)
					.with_height(ImageBoxSizeValue::Fill)
					.with_texture(backgrounds[0].name())
					.arclocked(),
			));
		}
		root = root.with_slot(SizeBox::new().with_navicability().arclocked());
		Self { root, backgrounds }
	}
}

impl Widget for Loading {
	fn get_image_ids(&self) -> Vec<engine::asset::Id> {
		self.backgrounds.clone()
	}
}

impl AsRAUI for Loading {
	fn as_raui(&self) -> WidgetComponent {
		self.root.as_raui()
	}
}
