use crate::engine::network;

mod handshake;
pub use handshake::*;

pub fn register_types(builder: &mut network::Builder) {
	Handshake::register(builder);
}
