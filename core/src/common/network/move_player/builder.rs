use std::sync::{RwLock, Weak};

use engine::socknet::stream::{
	self,
	kind::send::{self, Datagram},
};

use crate::entity;

pub struct Builder {
	pub entity_world: Weak<RwLock<entity::World>>,
}

impl stream::Identifier for Builder {
	fn unique_id() -> &'static str {
		"move_player"
	}
}

impl stream::send::Builder for Builder {
	type Opener = stream::datagram::Opener;
}

impl stream::recv::Builder for Builder {
	type Extractor = stream::datagram::Extractor;
	type Receiver = super::recv::Handler;
}
