use engine::network;

mod handshake;
pub use handshake::*;

mod connection;

pub fn register_types(builder: &mut network::Builder) {
	use crate::server::user;
	let auth_cache = user::pending::AuthCache::default().arclocked();
	Handshake::register(builder, &auth_cache);
	connection::register_bonus_processors(builder, &auth_cache);
}
