pub mod packet;

use crate::app::state::ArcLockMachine;
use engine::network::{mode, Builder};
fn create_builder(app_state: &ArcLockMachine) -> Builder {
	let mut net_builder = Builder::default().with_port(25565);
	packet::register_types(&mut net_builder, &app_state);
	net_builder
}

pub fn create_with_args(app_state: &ArcLockMachine) -> Builder {
	create_builder(&app_state).with_args()
}

pub fn create<TModeSet: Into<mode::Set>>(app_state: &ArcLockMachine, modes: TModeSet) -> Builder {
	create_builder(&app_state).with_mode(modes)
}
