pub use engine::input::{self, *};

pub static ACTION_TOGGLE_DEBUG_CMDS: &'static str = "ToggleDebugCommands";
pub static ACTION_TOGGLE_CHUNK_BOUNDARIES: &'static str = "ToggleChunkBoundaries";
pub static ACTION_SWAP_CAMERA_POV: &'static str = "SwapCameraPOV";

pub static AXIS_STRAFE: &'static str = "Strafe";
pub static AXIS_MOVE: &'static str = "Move";
pub static AXIS_FLY: &'static str = "Fly";
pub static AXIS_LOOK_HORIZONTAL: &'static str = "LookHorizontal";
pub static AXIS_LOOK_VERTICAL: &'static str = "LookVertical";

pub fn init() -> ArcLockUser {
	use prelude::{Source::Keyboard, *};
	input::set_config(
		Config::default()
			.add_action(ACTION_TOGGLE_DEBUG_CMDS, Kind::Button)
			.add_action(ACTION_TOGGLE_CHUNK_BOUNDARIES, Kind::Button)
			.add_action(ACTION_SWAP_CAMERA_POV, Kind::Button)
			.add_action(AXIS_STRAFE, Kind::Axis)
			.add_action(AXIS_MOVE, Kind::Axis)
			.add_action(AXIS_FLY, Kind::Axis)
			.add_action(AXIS_LOOK_HORIZONTAL, Kind::Axis)
			.add_action(AXIS_LOOK_VERTICAL, Kind::Axis)
			// The only layout is the default layout right now
			.add_layout(LayoutId::default())
			.add_action_set(
				Some("ApplicationActions"),
				ActionSet::default().with(
					LayoutId::default(),
					ActionMap::default()
						.bind(ACTION_TOGGLE_DEBUG_CMDS, Keyboard(Backslash))
						.bind(ACTION_TOGGLE_CHUNK_BOUNDARIES, Keyboard(F3)),
				),
			)
			.add_action_set(
				Some("CharacterControls"),
				ActionSet::default().with(
					LayoutId::default(),
					ActionMap::default()
						.bind(ACTION_SWAP_CAMERA_POV, Keyboard(F5))
						.bind(
							AXIS_MOVE,
							[(
								device::Kind::Keyboard,
								((Keyboard(W) + Multiplier(1.0))
									+ (Keyboard(S) + Multiplier(-1.0)))
								.with_behavior(Average)
								.with_behavior(Multiplier(2.0)),
							)],
						)
						.bind(
							AXIS_STRAFE,
							((Keyboard(A) + Multiplier(-1.0)) + (Keyboard(D) + Multiplier(1.0)))
								.with_behavior(Average)
								.with_behavior(Multiplier(2.0)),
						)
						.bind(
							AXIS_FLY,
							((Keyboard(E) + Multiplier(1.0)) + (Keyboard(Q) + Multiplier(-1.0)))
								.with_behavior(Average)
								.with_behavior(Multiplier(2.0)),
						)
						.bind(
							AXIS_LOOK_HORIZONTAL,
							[
								(
									device::Kind::Mouse,
									Source::Mouse(Mouse::Move(MouseX))
										+ ScreenPositionDelta + Multiplier(-3.0),
								),
								(
									device::Kind::Keyboard,
									((Keyboard(Numpad4) + Multiplier(1.0))
										+ (Keyboard(Numpad6) + Multiplier(-1.0)))
									.with_behavior(Average)
									.with_behavior(Multiplier(2.0))
									.with_behavior(Multiplier(0.05)),
								),
							],
						)
						.bind(
							AXIS_LOOK_VERTICAL,
							[
								(
									device::Kind::Mouse,
									Source::Mouse(Mouse::Move(MouseY)) + ScreenPositionDelta,
								),
								(
									device::Kind::Keyboard,
									((Keyboard(Numpad5) + Multiplier(1.0))
										+ (Keyboard(Numpad8) + Multiplier(-1.0)))
									.with_behavior(Average)
									.with_behavior(Multiplier(2.0))
									.with_behavior(Multiplier(0.05)),
								),
							],
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
