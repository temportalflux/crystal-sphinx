use std::net::SocketAddr;

#[derive(Clone, Copy)]
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
