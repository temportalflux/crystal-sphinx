use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

use chrono::{DateTime, Utc};
use socknet::stream::{self};

use crate::entity;

pub struct Builder {
	pub entity_world: Weak<RwLock<entity::World>>,
	pub sequencer: Arc<RwLock<HashMap<SocketAddr, DateTime<Utc>>>>,
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
