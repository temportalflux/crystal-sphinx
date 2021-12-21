use crate::{account, entity::component::net};
use engine::utility::AnyError;
use serde::{Deserialize, Serialize};

/// Indicates that an entity is controlled by a given account/user.
/// Use in conjunction with `net::Owner` to determine if the entity is
/// controlled by the local player and what account it is that controls it.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct User {
	account_id: account::Id,
}

impl std::fmt::Display for User {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "net::User({})", self.account_id)
	}
}

impl User {
	pub fn new(id: account::Id) -> Self {
		Self { account_id: id }
	}

	pub fn id(&self) -> &account::Id {
		&self.account_id
	}
}

impl net::Replicated for User {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::User"
	}

	fn serialize(&self) -> Result<Vec<u8>, AnyError> {
		Ok(rmp_serde::to_vec(&self)?)
	}
}

impl std::convert::TryFrom<Vec<u8>> for User {
	type Error = rmp_serde::decode::Error;
	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		net::deserialize::<Self>(&bytes)
	}
}
