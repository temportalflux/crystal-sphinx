use crate::{
	engine::{asset::statics, ui::*},
	ui,
};

pub fn root(mut _context: WidgetContext) -> WidgetNode {
	WidgetNode::Component(
		make_widget!(nav_content_box).listed_slot(
			make_widget!(size_box)
				.with_props(NavItemActive)
				.with_props(ContentBoxItemLayout {
					anchors: Rect {
						left: 0.5,
						right: 0.5,
						top: 0.5,
						bottom: 0.5,
					},
					align: Vec2 { x: 0.5, y: 0.5 },
					..Default::default()
				})
				.with_props(SizeBoxProps {
					width: SizeBoxSizeValue::Exact(250.0),
					height: SizeBoxSizeValue::Exact(150.0),
					..Default::default()
				})
				.named_slot(
					"content",
					make_widget!(ui::button::styled::widget).named_slot(
						"content",
						make_widget!(text_box)
							.with_props(TextBoxProps {
								text: "Button!".to_owned(),
								font: statics::font::unispace::REGULAR.at_size(30.0),
								color: Color {
									r: 0.0,
									g: 0.0,
									b: 0.0,
									a: 1.0,
								},
								..Default::default()
							})
							.with_props(ContentBoxItemLayout {
								anchors: Rect {
									left: 0.2,
									right: 0.5,
									top: 0.2,
									bottom: 0.5,
								},
								..Default::default()
							}),
					),
				),
		),
	)
}
