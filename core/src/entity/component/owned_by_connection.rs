use engine::utility::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct OwnedByConnection {
	/// The connection address this entity is owned/controlled by
	address: SocketAddr,
}

impl super::Component for OwnedByConnection {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::OwnedByConnection"
	}

	fn display_name() -> &'static str {
		"Owned By Connection"
	}

	fn registration() -> super::Registration<Self>
	where
		Self: Sized,
	{
		use super::binary::Registration as binary;
		use super::debug::Registration as debug;
		use super::network::Registration as network;
		super::Registration::<Self>::default()
			.with_ext(binary::from::<Self>())
			.with_ext(debug::from::<Self>())
			.with_ext(network::from::<Self>())
	}
}

impl std::fmt::Display for OwnedByConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "OwnedByConnection(address={})", self.address)
	}
}

impl OwnedByConnection {
	pub fn new(address: SocketAddr) -> Self {
		Self { address }
	}

	pub fn address(&self) -> &SocketAddr {
		&self.address
	}
}

impl super::network::Replicatable for OwnedByConnection {
	fn on_replication(&mut self, replicated: &Self, _is_locally_owned: bool) {
		*self = *replicated;
	}
}

impl super::binary::Serializable for OwnedByConnection {
	fn serialize(&self) -> Result<Vec<u8>> {
		super::binary::serialize(&self)
	}
	fn deserialize(bytes: Vec<u8>) -> Result<Self> {
		super::binary::deserialize::<Self>(&bytes)
	}
}

impl super::debug::EguiInformation for OwnedByConnection {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!("IP Address: {}", self.address));
	}
}
