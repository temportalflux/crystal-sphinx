pub mod binary;
mod camera;
pub use camera::*;
pub mod chunk;
pub mod debug;
pub mod network;
mod orientation;
pub use orientation::*;
mod owned_by_account;
pub use owned_by_account::*;
mod owned_by_connection;
pub use owned_by_connection::*;
pub mod physics;
mod registry;
pub use registry::*;

pub trait Component: hecs::Component {
	fn unique_id() -> &'static str;
	fn display_name() -> &'static str;
	fn registration() -> Registration<Self>
	where
		Self: Sized,
	{
		Registration::<Self>::default()
	}
}

pub fn register_types() {
	let mut registry = Registry::write();
	registry.register::<Camera>();
	registry.register::<chunk::Relevancy>();
	registry.register::<chunk::TicketOwner>();
	registry.register::<network::Replicated>();
	registry.register::<Orientation>();
	registry.register::<OwnedByAccount>();
	registry.register::<OwnedByConnection>();
	registry.register::<physics::linear::Position>();
	registry.register::<physics::linear::Velocity>();
}
