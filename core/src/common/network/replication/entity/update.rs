use crate::entity::component::binary::SerializedEntity;
use engine::channels::future::{Receiver, Sender};
use serde::{Deserialize, Serialize};

/// Async channel to send entity updates to be replicated to some client.
pub type SendUpdate = Sender<Update>;
/// Async channel to receive entity updates to be replicated to some client.
pub type RecvUpdate = Receiver<Update>;

/// An update to be replicated to some client.
#[derive(Clone, Serialize, Deserialize)]
pub enum Update {
	/// An entity is now relevant to the client and should be replicated.
	Relevant(SerializedEntity),
	/// A relevant entity has changed and should be replicated.
	Update(SerializedEntity),
	/// An entity was relevant and is no longer relevant to that client.
	Irrelevant(hecs::Entity),
	/// An entity was destroyed while it was relevant to some client.
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
