use engine::utility::AnyError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct OwnedByConnection {
	/// The connection address this entity is owned/controlled by
	address: SocketAddr,
	/// True when the entity has been replicated to its owner/connection
	has_been_replicated: bool,
}

impl super::Component for OwnedByConnection {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::OwnedByConnection"
	}

	fn display_name() -> &'static str {
		"Owned By Connection"
	}
}

impl std::fmt::Display for OwnedByConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "OwnedByConnection(address={})", self.address)
	}
}

impl OwnedByConnection {
	pub fn new(address: SocketAddr) -> Self {
		Self {
			address,
			has_been_replicated: false,
		}
	}

	pub fn address(&self) -> &SocketAddr {
		&self.address
	}

	pub(crate) fn has_been_replicated(&self) -> bool {
		self.has_been_replicated
	}

	pub(crate) fn mark_as_replicated(&mut self) {
		self.has_been_replicated = true;
	}
}

impl super::binary::Serializable for OwnedByConnection {
	fn serialize(&self) -> Result<Vec<u8>, AnyError> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for OwnedByConnection {
	type Error = rmp_serde::decode::Error;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		super::binary::deserialize::<Self>(&bytes)
	}
}
