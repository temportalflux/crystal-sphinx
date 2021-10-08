use engine::{
	asset::statics,
	ui::{
		oui::{widget, *},
		raui::*,
	},
};

pub struct Home {
	text: widget::Text,
}

impl Home {
	pub fn new() -> Self {
		Self {
			text: widget::Text::new()
				.with_text("!HOME!".to_owned())
				.with_font(statics::font::unispace::REGULAR.at_size(30.0))
				.with_align_horizontal(TextBoxHorizontalAlign::Center)
				.with_align_vertical(TextBoxVerticalAlign::Middle),
		}
	}
}

impl Widget for Home {}

impl AsRAUI for Home {
	fn as_raui(&self) -> WidgetComponent {
		make_widget!(nav_content_box)
			.listed_slot(make_widget!(image_box).with_props(ImageBoxProps {
				material: ImageBoxMaterial::Color(ImageBoxColor {
					color: Color {
						r: 0.05,
						g: 0.05,
						b: 0.05,
						a: 1.0,
					},
					..Default::default()
				}),
				..Default::default()
			}))
			.listed_slot(self.text.as_raui().with_props(ContentBoxItemLayout {
				anchors: Rect {
					left: 0.5,
					right: 0.5,
					top: 0.5,
					bottom: 0.5,
				},
				..Default::default()
			}))
	}
}
