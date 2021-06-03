use crate::{engine::ui::*, ui};

pub fn root(mut _context: WidgetContext) -> WidgetNode {
	WidgetNode::Component(
		make_widget!(content_box).listed_slot(
			make_widget!(size_box)
				.with_props(ContentBoxItemLayout {
					anchors: Rect {
						left: 0.0,
						right: 0.0,
						top: 0.0,
						bottom: 0.0,
					},
					align: Vec2 { x: 0.5, y: 0.5 },
					..Default::default()
				})
				.with_props(SizeBoxProps {
					width: SizeBoxSizeValue::Exact(250.0),
					height: SizeBoxSizeValue::Exact(150.0),
					..Default::default()
				})
				.named_slot("content", make_widget!(ui::button::button)),
		),
	)
}
