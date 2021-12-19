mod camera;
pub use camera::*;
pub mod net;
mod orientation;
pub use orientation::*;
mod position;
pub use position::*;

pub fn register_replicated_components() {
	let mut registry = net::Registry::write();
	registry.register::<Position>();
	registry.register::<Orientation>();
}
