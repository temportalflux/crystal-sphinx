use crate::{
	client,
	common::account,
	entity::component::{
		chunk,
		network::Replicated,
		physics::linear::{Position, Velocity},
		Camera, Orientation, OwnedByAccount, OwnedByConnection,
	},
};
use std::net::SocketAddr;

pub struct Server(hecs::EntityBuilder);
impl Server {
	pub fn new() -> Self {
		let mut builder = hecs::EntityBuilder::default();
		builder.add(Replicated::new_server());
		builder.add(Position::default());
		builder.add(Velocity::default());
		builder.add(Orientation::default());
		builder.add(chunk::TicketOwner::default().with_load_radius(5));
		builder.add(
			chunk::Relevancy::default()
				.with_radius(6) // TODO: This radius should match the radius in the graphics instance buffer
				.with_entity_radius(5),
		);
		Self(builder)
	}

	pub fn with_user_id(mut self, id: account::Id) -> Self {
		self.0.add(OwnedByAccount::new(id));
		self
	}

	pub fn with_address(mut self, address: SocketAddr) -> Self {
		self.0.add(OwnedByConnection::new(address));
		self
	}

	pub fn build(self) -> hecs::EntityBuilder {
		self.0
	}
}

/// Creates a builder of components that only need to be created on the owning-client,
/// returning only those types which do not already exist on the entity.
pub struct Client(hecs::EntityBuilder, bool);
impl Client {
	pub fn new(builder: hecs::EntityBuilder) -> Self {
		use engine::Application;
		let mut client = Self(builder, false);
		client.add_opt::<Camera>();
		client.add_opt_fn(|| {
			client::model::blender::Component::new(
				crate::CrystalSphinx::get_asset_id("entity/humanoid/default"),
				crate::CrystalSphinx::get_asset_id("entity/humanoid/textures/skin"),
			)
		});
		client
	}

	pub fn apply_to(builder: hecs::EntityBuilder) -> hecs::EntityBuilder {
		Self::new(builder).build()
	}

	fn has<T>(&self) -> bool
	where
		T: hecs::Component,
	{
		self.0.has::<T>()
	}

	fn add_opt<T>(&mut self)
	where
		T: hecs::Component + Default,
	{
		self.add_opt_fn(|| T::default());
	}

	fn add_opt_fn<T>(&mut self, constructor: impl FnOnce() -> T)
	where
		T: hecs::Component,
	{
		if !self.has::<T>() {
			self.0.add(constructor());
			self.1 = true;
		}
	}

	pub fn build(self) -> hecs::EntityBuilder {
		self.0
	}
}
