pub mod packet;
pub mod storage;
pub mod task;

use crate::{
	app::state::ArcLockMachine, entity::ArcLockEntityWorld, network::storage::ArcLockStorage,
};
use engine::network::{mode, Builder};
fn create_builder(
	app_state: &ArcLockMachine,
	storage: &ArcLockStorage,
	entity_world: &ArcLockEntityWorld,
) -> Builder {
	let mut net_builder = Builder::default().with_port(25565);
	packet::register_types(&mut net_builder, &app_state, &storage, &entity_world);
	net_builder
}

pub fn create<TModeSet: Into<mode::Set>>(
	modes: TModeSet,
	app_state: &ArcLockMachine,
	storage: &ArcLockStorage,
	entity_world: &ArcLockEntityWorld,
) -> Builder {
	create_builder(&app_state, &storage, &entity_world).with_mode(modes)
}
