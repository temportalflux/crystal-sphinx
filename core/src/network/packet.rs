use engine::network;

mod move_player;
pub use move_player::*;

pub mod replicate_entity;

mod replicate_world;
pub use replicate_world::*;

pub fn register_types(
	builder: &mut network::Builder,
	app_state: &crate::app::state::ArcLockMachine,
	storage: &super::storage::ArcLockStorage,
	entity_world: &crate::entity::ArcLockEntityWorld,
) {
	use crate::network::storage::server::user;
	let auth_cache = user::pending::Cache::default().arclocked();
	let active_cache = user::active::Cache::default().arclocked();
	replicate_entity::Packet::register(builder, &entity_world);
	ReplicateWorld::register(builder, &storage, &entity_world);
	MovePlayer::register(builder, &entity_world);
}
