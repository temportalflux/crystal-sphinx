use crate::{
	account,
	entity::component::{chunk, Camera, Orientation, OwnedByAccount, OwnedByConnection, Position},
};
use std::net::SocketAddr;

pub struct Server(hecs::EntityBuilder);
impl Server {
	pub fn new() -> Self {
		let mut builder = hecs::EntityBuilder::default();
		builder.add(Position::default());
		builder.add(Orientation::default());
		builder.add(chunk::TicketOwner::default().with_load_radius(2));
		builder.add(chunk::Relevancy::default().with_radius(5)); // TODO: This radius should match the radius in the graphics instance buffer
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
pub struct Client<'e>(Option<hecs::EntityRef<'e>>, hecs::EntityBuilder, bool);
impl<'e> Client<'e> {
	fn has<T>(&self) -> bool
	where
		T: hecs::Component,
	{
		match self.0 {
			Some(entity_ref) => entity_ref.has::<T>(),
			None => false,
		}
	}

	fn add_opt<T>(&mut self)
	where
		T: hecs::Component + Default,
	{
		if !self.has::<T>() {
			self.1.add(T::default());
			self.2 = true;
		}
	}

	pub(crate) fn build(self) -> Option<hecs::EntityBuilder> {
		match self.2 {
			true => Some(self.1),
			false => None,
		}
	}
}
impl<'e> From<Option<hecs::EntityRef<'e>>> for Client<'e> {
	fn from(entity_ref: Option<hecs::EntityRef<'e>>) -> Self {
		let mut client = Self(entity_ref, hecs::EntityBuilder::default(), false);
		client.add_opt::<Camera>();
		client
	}
}
