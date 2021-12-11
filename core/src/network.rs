pub mod packet;
pub mod storage;
pub mod task;

use crate::{app::state::ArcLockMachine, network::storage::ArcLockStorage};
use engine::network::{mode, Builder};
fn create_builder(app_state: &ArcLockMachine, storage: &ArcLockStorage) -> Builder {
	let mut net_builder = Builder::default().with_port(25565);
	packet::register_types(&mut net_builder, &app_state, &storage);
	net_builder
}

pub fn create<TModeSet: Into<mode::Set>>(
	modes: TModeSet,
	app_state: &ArcLockMachine,
	storage: &ArcLockStorage,
) -> Builder {
	create_builder(&app_state, &storage).with_mode(modes)
}
