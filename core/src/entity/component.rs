pub mod binary;
mod camera;
pub use camera::*;
pub mod chunk;
pub mod debug;
mod orientation;
pub use orientation::*;
mod owned_by_account;
pub use owned_by_account::*;
mod owned_by_connection;
pub use owned_by_connection::*;
mod position;
pub use position::*;
mod registry;
pub use registry::*;
mod replicated;
pub use replicated::*;

pub trait Component: hecs::Component {
	fn unique_id() -> &'static str;
	fn display_name() -> &'static str;
}

pub fn register_replicated_components() {
	let mut registry = Registry::write();
	registry.add(Registration::<Camera>::default().with_debug());
	registry.add(Registration::<chunk::Relevancy>::default());
	registry.add(Registration::<chunk::TicketOwner>::default());
	registry.add(Registration::<Orientation>::default().with_binary_serialization().with_debug());
	registry.add(Registration::<OwnedByAccount>::default().with_binary_serialization());
	registry.add(Registration::<OwnedByConnection>::default().with_binary_serialization());
	registry.add(
		Registration::<Position>::default()
			.with_binary_serialization()
			.with_debug(),
	);
}
