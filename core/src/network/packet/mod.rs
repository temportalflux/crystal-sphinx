use crate::engine::network;

mod handshake;
pub use handshake::*;

pub fn register_types(builder: &mut network::Builder) {
	use crate::server::user;
	let auth_cache = user::pending::AuthCache::default().arclocked();
	Handshake::register(builder, &auth_cache);
}
