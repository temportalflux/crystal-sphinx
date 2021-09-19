use engine::network;

mod handshake;
pub use handshake::*;

mod connection;

pub fn register_types(builder: &mut network::Builder) {
	use crate::server::user;
	let auth_cache = user::pending::Cache::default().arclocked();
	let active_cache = user::active::Cache::default().arclocked();
	Handshake::register(builder, &auth_cache, &active_cache);
	connection::register_bonus_processors(builder, &auth_cache, &active_cache);
}
