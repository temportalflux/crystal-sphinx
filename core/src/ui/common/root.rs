use crate::ui;
use engine::{
	asset::statics,
	ui::{raui::*, ContextContainer},
};

pub fn root(ctx: WidgetContext) -> WidgetNode {
	let app_state = ctx.process_context.get::<ContextContainer>();
	log::debug!("app_state: {}", app_state.is_some());
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
					make_widget!(ui::common::button::styled::widget).named_slot(
						"content",
						make_widget!(text_box)
							.with_props(TextBoxProps {
								text: "Connect to Server".to_owned(),
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
