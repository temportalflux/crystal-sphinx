pub use engine::input::{self, *};

pub static ACTION_TOGGLE_DEBUG_CMDS: &'static str = "ToggleDebugCommands";

pub fn init() {
	use engine::input::{
		action::Action,
		binding::{ActionMap, ActionSet, ActionSetId, LayoutId, Source::*},
		source::{Key::*, Kind},
	};
	input::write()
		.add_users(1)
		.add_action(ACTION_TOGGLE_DEBUG_CMDS, Action::new(Kind::Button))
		.add_layout(LayoutId::default())
		.add_action_set(
			ActionSetId::default(),
			ActionSet::default().with(
				LayoutId::default(),
				ActionMap::default()
					.bind(ACTION_TOGGLE_DEBUG_CMDS, vec![Keyboard(Backslash).bound()]),
			),
		)
		.enable_action_set_for_all(ActionSetId::default());
}
