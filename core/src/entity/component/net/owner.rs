use crate::entity::component::net;
use engine::utility::AnyError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Owner {
	/// The connection address this entity is owned/controlled by
	address: SocketAddr,
	/// True when the entity has been replicated to its owner/connection
	has_been_replicated: bool,
}

impl std::fmt::Display for Owner {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "net::Owner(address={})", self.address)
	}
}

impl Owner {
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

impl net::Replicated for Owner {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::net::Owner"
	}

	fn serialize(&self) -> Result<Vec<u8>, AnyError> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for Owner {
	type Error = rmp_serde::decode::Error;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		net::deserialize::<Self>(&bytes)
	}
}
