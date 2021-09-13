use crate::ui::state;
use engine::ui::*;
use serde::{Deserialize, Serialize};

#[derive(PropsData, Debug, Clone, Serialize, Deserialize)]
pub struct Style {
	background_color: state::Var<Color>,
	border_color: state::Var<Color>,
	inner_color_top_left: state::Var<Color>,
	inner_color_bottom_right: state::Var<Color>,
}

impl Default for Style {
	fn default() -> Self {
		Self {
			background_color: state::Var::<Color>::new(Color {
				r: 0.435,
				g: 0.435,
				b: 0.435,
				a: 1.0,
			})
			.with(
				state::Flag::Disabled.into(),
				Color {
					r: 0.173,
					g: 0.173,
					b: 0.173,
					a: 1.0,
				},
			),
			border_color: state::Var::<Color>::new(Color {
				r: 0.03,
				g: 0.03,
				b: 0.03,
				a: 1.0,
			})
			.with(
				state::Flag::Active.into(),
				Color {
					r: 0.7,
					g: 0.7,
					b: 0.7,
					a: 1.0,
				},
			),
			inner_color_top_left: state::Var::<Color>::new(Color {
				r: 0.8,
				g: 0.8,
				b: 0.8,
				a: 0.5,
			})
			.with(
				state::Flag::Disabled.into(),
				Color {
					r: 0.0,
					g: 0.0,
					b: 0.0,
					a: 0.0,
				},
			)
			.with(
				state::Flag::Hovered.into(),
				Color {
					r: 1.0,
					g: 1.0,
					b: 1.0,
					a: 0.8,
				},
			),
			inner_color_bottom_right: state::Var::<Color>::new(Color {
				r: 0.0,
				g: 0.0,
				b: 0.0,
				a: 0.7,
			})
			.with(
				state::Flag::Disabled.into(),
				Color {
					r: 0.0,
					g: 0.0,
					b: 0.0,
					a: 0.0,
				},
			)
			.with(
				state::Flag::Hovered.into(),
				Color {
					r: 0.0,
					g: 0.0,
					b: 0.0,
					a: 0.8,
				},
			),
		}
	}
}

// use_button_notified_state - exposes `ButtonProps` as a valid state for this widget, which contains the stateful behavior data
#[pre_hooks(use_button_notified_state)]
pub fn widget(mut ctx: WidgetContext) -> WidgetNode {
	let style = ctx.props.read_cloned_or_default::<Style>();
	let flags = state::Flag::from_ctx(&ctx);
	unpack_named_slots!(ctx.named_slots => { content });

	let border_size = Rect {
		left: 5.0,
		top: 5.0,
		right: 5.0,
		bottom: 5.0,
	};

	let border = make_widget!(image_box).with_props(ImageBoxProps {
		material: ImageBoxMaterial::Color(ImageBoxColor {
			color: *style.border_color.first(flags),
			scaling: ImageBoxImageScaling::Frame(ImageBoxFrame {
				source: border_size,
				destination: border_size,
				frame_only: true,
				..Default::default()
			}),
		}),
		..Default::default()
	});

	let background = make_widget!(image_box).with_props(ImageBoxProps {
		material: ImageBoxMaterial::Color(ImageBoxColor {
			color: *style.background_color.first(flags),
			..Default::default()
		}),
		..Default::default()
	});

	let inner_highlight_top_left = make_widget!(image_box).with_props(ImageBoxProps {
		material: ImageBoxMaterial::Color(ImageBoxColor {
			color: *style.inner_color_top_left.first(flags),
			scaling: ImageBoxImageScaling::Frame(ImageBoxFrame {
				source: Rect {
					left: 5.0,
					top: 5.0,
					right: 0.0,
					bottom: 0.0,
				},
				destination: Rect {
					left: 5.0,
					top: 5.0,
					right: 0.0,
					bottom: 0.0,
				},
				frame_only: true,
				..Default::default()
			}),
		}),
		..Default::default()
	});

	let inner_highlight_bottom_right = make_widget!(image_box).with_props(ImageBoxProps {
		material: ImageBoxMaterial::Color(ImageBoxColor {
			color: *style.inner_color_bottom_right.first(flags),
			scaling: ImageBoxImageScaling::Frame(ImageBoxFrame {
				source: Rect {
					left: 0.0,
					top: 0.0,
					right: 5.0,
					bottom: 10.0,
				},
				destination: Rect {
					left: 0.0,
					top: 0.0,
					right: 5.0,
					bottom: 10.0,
				},
				frame_only: true,
				..Default::default()
			}),
		}),
		..Default::default()
	});

	WidgetNode::Component(
		make_widget!(button)
			.with_props(NavItemActive)
			.with_props(ButtonNotifyProps(ctx.id.to_owned().into()))
			.named_slot(
				"content",
				make_widget!(content_box).listed_slot(border).listed_slot(
					make_widget!(content_box)
						.with_props(ContentBoxItemLayout {
							margin: border_size,
							..Default::default()
						})
						.listed_slot(background)
						.listed_slot(inner_highlight_top_left)
						.listed_slot(inner_highlight_bottom_right)
						.listed_slot(content),
				),
			),
	)
}
