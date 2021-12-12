use engine::network;

mod handshake;
pub use handshake::*;

mod connection;

pub fn register_types(
	builder: &mut network::Builder,
	app_state: &crate::app::state::ArcLockMachine,
	storage: &super::storage::ArcLockStorage,
) {
	use crate::network::storage::server::user;
	let auth_cache = user::pending::Cache::default().arclocked();
	let active_cache = user::active::Cache::default().arclocked();
	Handshake::register(builder, &auth_cache, &active_cache, &app_state, &storage);
	connection::register_bonus_processors(
		builder,
		&auth_cache,
		&active_cache,
		&app_state,
		&storage,
	);
}
