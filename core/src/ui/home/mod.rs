use engine::{
	asset::statics,
	ui::{
		oui::{widget, *},
		raui::*,
	},
};

pub struct Home {}

impl Home {
	pub fn new() -> Self {
		Self {}
	}
}

impl Widget for Home {}

impl AsRAUI for Home {
	fn as_raui(&self) -> WidgetComponent {
		make_widget!(nav_content_box).listed_slot(
			make_widget!(image_box)
				.with_props(ImageBoxProps {
					width: ImageBoxSizeValue::Exact(400.0),
					height: ImageBoxSizeValue::Exact(100.0),
					material: ImageBoxMaterial::Image(ImageBoxImage {
						id: "textures/ui/title".to_owned(),
						..Default::default()
					}),
					..Default::default()
				})
				.with_props(ContentBoxItemLayout {
					anchors: Rect {
						left: 0.3,
						right: 0.5,
						top: 0.01,
						bottom: 0.5,
					},
					..Default::default()
				}),
		)
	}
}
