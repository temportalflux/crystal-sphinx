use crate::entity::component::{net, Camera, Orientation, Position};
use std::net::SocketAddr;

pub struct Server(hecs::EntityBuilder);

impl Server {
	pub fn new() -> Self {
		let mut builder = hecs::EntityBuilder::default();
		builder.add(Position::default()).add(Orientation::default());
		Self(builder)
	}

	pub fn with_address(mut self, address: SocketAddr) -> Self {
		self.0.add(net::Owner::new(address));
		self
	}

	pub fn build(&mut self) -> hecs::BuiltEntity<'_> {
		self.0.build()
	}
}

pub fn client_only() -> hecs::EntityBuilder {
	let mut builder = hecs::EntityBuilder::default();
	builder.add(Camera::default());
	builder
}
