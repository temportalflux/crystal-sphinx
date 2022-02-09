use crate::entity::component::binary::SerializedEntity;
use serde::{Deserialize, Serialize};

mod builder;
pub use builder::*;

pub mod recv;
pub mod send;

pub type Channel = async_channel::Receiver<Update>;

#[derive(Clone, Serialize, Deserialize)]
pub enum Update {
	Relevant(SerializedEntity),
	Update(SerializedEntity),
	Irrelevant(hecs::Entity),
	Destroyed(hecs::Entity),
}

impl std::fmt::Debug for Update {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Relevant(serialized) => {
				write!(f, "Relevant({})", serialized.entity.id())
			}
			Self::Update(serialized) => {
				write!(f, "Update({})", serialized.entity.id())
			}
			Self::Irrelevant(entity) => write!(f, "Irrelevant({})", entity.id()),
			Self::Destroyed(entity) => write!(f, "Destroyed({})", entity.id()),
		}
	}
}
