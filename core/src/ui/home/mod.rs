use engine::ui::{
	oui::{
		widget::{container::content_box, ImageBox, SizeBox},
		AsRAUI, Widget,
	},
	raui::*,
};

pub struct Home {
	root: content_box::Container,
}

impl Home {
	pub fn new() -> Self {
		let root = content_box::Container::new()
			.with_slot(
				content_box::Slot::from(
					ImageBox::new()
						.with_width(ImageBoxSizeValue::Exact(400.0))
						.with_height(ImageBoxSizeValue::Exact(100.0))
						.with_texture("textures/ui/title".to_owned())
						.arclocked(),
				)
				.with_layout(ContentBoxItemLayout {
					anchors: Rect {
						left: 0.3,
						right: 0.5,
						top: 0.01,
						bottom: 0.5,
					},
					..Default::default()
				}),
			)
			.with_slot(SizeBox::new().with_navicability().arclocked());
		Self { root }
	}
}

impl Widget for Home {}

impl AsRAUI for Home {
	fn as_raui(&self) -> WidgetComponent {
		self.root.as_raui()
	}
}
