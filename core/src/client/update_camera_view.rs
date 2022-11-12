use super::model::PlayerModel;
use crate::{
	common::account,
	entity::{self, component},
};
use engine::{input, utility::ValueSet, Engine, EngineSystem};
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::OwnedByAccount,
	&'c mut component::Camera,
	&'c mut PlayerModel,
)>;

pub struct UpdateCameraView {
	world: Weak<RwLock<entity::World>>,
	account_id: account::Id,
	input_action: input::action::WeakLockState,
}

impl UpdateCameraView {
	pub fn new(systems: &Arc<ValueSet>) -> anyhow::Result<Arc<Self>> {
		let world = Arc::downgrade(&systems.get_arclock::<entity::World>().unwrap());
		let arc_user = systems.get_arclock::<input::User>().unwrap();

		let input_action =
			crate::input::User::get_action_in(&arc_user, crate::input::ACTION_SWAP_CAMERA_POV)
				.unwrap();
		let account_id = crate::client::account::Manager::read()?
			.active_account()?
			.id();
		Ok(Arc::new(Self {
			world,
			account_id,
			input_action,
		}))
	}
	
	pub fn update(&self) {
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
		for (_entity, (entity_user, camera, model)) in query_bundle.query(&world).iter() {
			// Only control the entity which is owned by the local player
			if *entity_user.id() != self.account_id {
				continue;
			}
			let next_point_of_view = camera.view().next();
			camera.set_view(next_point_of_view);
			model.set_perspective(next_point_of_view.perspective());
		}
	}
}
