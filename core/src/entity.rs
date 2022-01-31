use std::sync::{Arc, RwLock, Weak};

pub mod archetype;
pub mod component;
pub mod system;

pub use hecs::World;
/// Alias for Arc<RwLock<[`World`](hecs::World)>>
pub type ArcLockEntityWorld = Arc<RwLock<World>>;

/// Adds a listener to clear all the entities from the world
/// when the application leaves the [`InGame`](crate::app::state::State::InGame) state.
pub fn add_state_listener(
	app_state: &Arc<RwLock<crate::app::state::Machine>>,
	entity_world: Weak<RwLock<World>>,
) {
	use crate::app::state::{OperationKey, State::InGame, Transition::Exit};
	app_state.write().unwrap().add_async_callback(
		OperationKey(Some(InGame), Some(Exit), None),
		move |_operation| {
			log::info!(target: "entity", "Detected Exit(InGame) transition, clearing all entities from the world.");
			let weak_world = entity_world.clone();
			async move {
				profiling::scope!("clear-entities");
				if let Some(entity_world) = weak_world.upgrade() {
					let mut world = entity_world.write().unwrap();
					world.clear();
				}
				Ok(())
			}
		},
	);
}
