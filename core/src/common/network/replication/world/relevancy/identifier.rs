use std::sync::Arc;

use socknet::stream;

use crate::common::network::replication::world::relevancy::{client, server};

pub struct Identifier {
	pub client: Arc<client::AppContext>,
	pub server: Arc<server::AppContext>,
}

impl stream::Identifier for Identifier {
	type SendBuilder = server::AppContext;
	type RecvBuilder = client::AppContext;
	fn unique_id() -> &'static str {
		"replication::chunk-relv"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.server
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.client
	}
}
