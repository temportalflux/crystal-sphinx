use crate::account;
use engine::utility::Result;
use serde::{Deserialize, Serialize};

/// Indicates that an entity is controlled by a given account/user.
/// Use in conjunction with `net::Owner` to determine if the entity is
/// controlled by the local player and what account it is that controls it.
#[derive(Clone, Serialize, Deserialize)]
pub struct OwnedByAccount {
	account_id: account::Id,
}

impl super::Component for OwnedByAccount {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::OwnedByAccount"
	}

	fn display_name() -> &'static str {
		"Owned By Account"
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

impl std::fmt::Display for OwnedByAccount {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "net::User({})", self.account_id)
	}
}

impl OwnedByAccount {
	pub fn new(id: account::Id) -> Self {
		Self { account_id: id }
	}

	pub fn id(&self) -> &account::Id {
		&self.account_id
	}
}

impl super::network::Replicatable for OwnedByAccount {
	fn on_replication(&mut self, replicated: &Self, _is_locally_owned: bool) {
		*self = replicated.clone();
	}
}

impl super::binary::Serializable for OwnedByAccount {
	fn serialize(&self) -> Result<Vec<u8>> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for OwnedByAccount {
	type Error = rmp_serde::decode::Error;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		super::binary::deserialize::<Self>(&bytes)
	}
}

impl super::debug::EguiInformation for OwnedByAccount {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!("Account ID: {}", self.account_id));
	}
}
