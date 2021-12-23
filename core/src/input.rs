pub use engine::input::{self, *};

pub static ACTION_TOGGLE_DEBUG_CMDS: &'static str = "ToggleDebugCommands";

pub static AXIS_STRAFE: &'static str = "Strafe";
pub static AXIS_MOVE: &'static str = "Move";
pub static BUTTON_FLY_UP: &'static str = "FlyUp";
pub static BUTTON_FLY_DOWN: &'static str = "FlyDown";
pub static AXIS_LOOK_HORIZONTAL: &'static str = "LookHorizontal";
pub static AXIS_LOOK_VERTICAL: &'static str = "LookVertical";

pub fn init() -> ArcLockUser {
	use prelude::{Source::Keyboard, *};
	input::set_config(
		Config::default()
			.add_action(ACTION_TOGGLE_DEBUG_CMDS, Kind::Button)
			.add_action(AXIS_STRAFE, Kind::Axis)
			.add_action(AXIS_MOVE, Kind::Axis)
			.add_action(BUTTON_FLY_UP, Kind::Button)
			.add_action(BUTTON_FLY_DOWN, Kind::Button)
			.add_action(AXIS_LOOK_HORIZONTAL, Kind::Axis)
			.add_action(AXIS_LOOK_VERTICAL, Kind::Axis)
			// The only layout is the default layout right now
			.add_layout(LayoutId::default())
			.add_action_set(
				Some("ApplicationActions"),
				ActionSet::default().with(
					LayoutId::default(),
					ActionMap::default().bind(ACTION_TOGGLE_DEBUG_CMDS, Keyboard(Backslash)),
				),
			)
			.add_action_set(
				Some("CharacterControls"),
				ActionSet::default().with(
					LayoutId::default(),
					ActionMap::default()
						.bind(
							AXIS_MOVE,
							(Keyboard(W) + Multiplier(1.0)) + (Keyboard(S) + Multiplier(-1.0)),
						)
						.bind(
							AXIS_STRAFE,
							(Keyboard(D) + Multiplier(1.0)) + (Keyboard(A) + Multiplier(-1.0)),
						)
						.bind(BUTTON_FLY_UP, Keyboard(E))
						.bind(BUTTON_FLY_DOWN, Keyboard(Q))
						.bind(
							AXIS_LOOK_HORIZONTAL,
							Source::Mouse(Mouse::Move(MouseX))
								+ ScreenPositionDelta + Multiplier(3.0),
						)
						.bind(
							AXIS_LOOK_VERTICAL,
							Source::Mouse(Mouse::Move(MouseY)) + ScreenPositionDelta,
						),
				),
			),
	);

	let arc_user = engine::input::create_user("Local");
	if let Ok(mut user) = arc_user.write() {
		user.enable_action_set(Some("ApplicationActions"));
		user.enable_action_set(Some("CharacterControls"));
	}

	arc_user
}
