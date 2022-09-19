use crate::{
	common::account,
	entity::{self, component},
};
use engine::{input, Engine, EngineSystem};
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> =
	hecs::PreparedQuery<(&'c component::OwnedByAccount, &'c mut component::Camera)>;

pub struct UpdateCameraView {
	world: Weak<RwLock<entity::World>>,
	account_id: account::Id,
	input_action: input::action::WeakLockState,
}

impl UpdateCameraView {
	pub fn create(
		world: Weak<RwLock<entity::World>>,
		arc_user: &input::ArcLockUser,
	) -> anyhow::Result<Option<Arc<RwLock<Self>>>> {
		let input_action =
			crate::input::User::get_action_in(&arc_user, crate::input::ACTION_SWAP_CAMERA_POV)
				.unwrap();
		let account_id = crate::client::account::Manager::read()?
			.active_account()?
			.id();
		let arc_self = Arc::new(RwLock::new(Self {
			world,
			account_id,
			input_action,
		}));
		// Run updates on the system as long as the object exists (i.e. while the app's state is `InGame`).
		if let Ok(mut engine) = Engine::get().write() {
			engine.add_weak_system(Arc::downgrade(&arc_self));
		}
		Ok(Some(arc_self))
	}
}

impl EngineSystem for UpdateCameraView {
	fn update(&mut self, _delta_time: std::time::Duration, _: bool) {
		profiling::scope!("subsystem:update_camera_view");

		if let Some(arc_state) = self.input_action.upgrade() {
			if let Ok(state) = arc_state.read() {
				// Only perform the update if the input button was pressed.
				// If it was not pressed, this is a no-op system.
				if !state.on_button_pressed() {
					return;
				}
			}
		}

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let world = arc_world.read().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (_entity, (entity_user, camera)) in query_bundle.query(&world).iter() {
			// Only control the entity which is owned by the local player
			if *entity_user.id() != self.account_id {
				continue;
			}
			camera.set_view(camera.view().next());
		}
	}
}
