use crate::engine::ui::*;

/* Button
	Default:
	- background = #6f6f6f
	- inner-top-left = opacity(28%)
	- inner-bottom-right = opacity(40%)
	- border = #000000
	Disabled (exclusive):
	- background = #2c2c2c
	- inner-top-left = opacity(0%)
	- inner-bottom-right = opacity(0%)
	Hovered:
	- inner-top-left = opacity(62%)
	- inner-bottom-right = opacity(66%)
	Selected: invalid state
	Pressed:
	- border = #ffffff
*/

// use_button_notified_state - exposes `ButtonProps` as a valid state for this widget, which contains the stateful behavior data
#[pre_hooks(use_button_notified_state)]
pub fn button(mut ctx: WidgetContext) -> WidgetNode {
	let ButtonProps {
		// if the button is hovered via mouse or navigated to view gamepad/keyboard
		selected: is_hovered,
		// if the button is currently pressed/active because of a click or key/gamepad button input
		trigger: is_active,
		..
	} = ctx.state.read_cloned_or_default();

	let background_color = match (is_hovered, is_active) {
		(true, _) => Color {
			r: 0.6,
			g: 0.6,
			b: 0.6,
			a: 1.0,
		},
		(_, true) => Color {
			r: 1.0,
			g: 1.0,
			b: 1.0,
			a: 1.0,
		},
		_ => Color {
			r: 0.435,
			g: 0.435,
			b: 0.435,
			a: 1.0,
		},
	};

	WidgetNode::Component(make_widget!(content_box).listed_slot(
		make_widget!(image_box).with_props(ImageBoxProps {
			material: ImageBoxMaterial::Color(ImageBoxColor {
				color: background_color,
				..Default::default()
			}),
			..Default::default()
		}),
	))
}
