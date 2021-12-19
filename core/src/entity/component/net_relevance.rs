use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Indicates that the entity is owned by the provided connection address.
#[derive(Clone, Copy)]
pub struct NetRelevance {
	address: SocketAddr,
}

impl Default for NetRelevance {
	fn default() -> Self {
		Self {
			address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
		}
	}
}

impl std::fmt::Display for NetRelevance {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "NetRelevance(address={})", self.address)
	}
}

impl NetRelevance {
	pub fn new(address: SocketAddr) -> Self {
		Self { address }
	}
}
