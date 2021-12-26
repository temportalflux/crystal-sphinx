mod camera;
pub use camera::*;
pub mod chunk;
pub mod net;
mod orientation;
pub use orientation::*;
mod position;
pub use position::*;
mod user;
pub use user::*;

pub fn register_replicated_components() {
	let mut registry = net::Registry::write();
	registry.register::<Position>();
	registry.register::<Orientation>();
	registry.register::<net::Owner>();
	registry.register::<User>();
}
