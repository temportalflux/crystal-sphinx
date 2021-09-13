use engine::ui::{ButtonProps, WidgetContext};
use enumset::{EnumSet, EnumSetType};

#[derive(Debug, EnumSetType)]
pub enum Flag {
	Disabled,
	/// also known as: focused
	Hovered,
	/// also known as: selected, pressed, triggered
	Active,
}

impl Flag {
	pub fn from_ctx(ctx: &WidgetContext) -> EnumSet<Flag> {
		use crate::ui::state::Props as StateProps;
		let ButtonProps {
			// if the button is hovered via mouse or navigated to view gamepad/keyboard
			selected: is_hovered,
			// if the button is currently pressed/active because of a click or key/gamepad button input
			trigger: is_active,
			..
		} = ctx.state.read_cloned_or_default();
		let StateProps { is_enabled } = ctx.state.read_cloned_or_default();
		[
			if !is_enabled {
				Some(Flag::Disabled)
			} else {
				None
			},
			if is_hovered {
				Some(Flag::Hovered)
			} else {
				None
			},
			if is_active { Some(Flag::Active) } else { None },
		]
		.iter()
		.fold(EnumSet::<Flag>::empty(), |set, &flag| match flag {
			Some(f) => set | f,
			None => set,
		})
	}
}
