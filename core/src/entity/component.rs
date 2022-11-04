pub mod binary;
mod camera;
pub use camera::*;
pub mod chunk;
pub mod debug;
pub mod network;
pub use crate::common::physics::component::Orientation;
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
	registry.register::<crate::common::physics::component::Position>();
	registry.register::<physics::linear::Velocity>();
	registry.register::<crate::common::physics::component::RigidBody>();
	registry.register::<crate::common::physics::component::RigidBodyHandle>();
	registry.register::<crate::common::physics::component::RigidBodyIsActive>();
	registry.register::<crate::common::physics::component::Collider>();
	registry.register::<crate::common::physics::component::ColliderHandle>();
	registry.register::<crate::common::physics::component::CollidingWith>();
	registry.register::<crate::client::physics::RenderCollider>();
	registry.register::<crate::client::model::blender::Component>();
	registry.register::<crate::client::model::PlayerModel>();
}
