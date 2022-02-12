use socknet::stream;
use std::sync::Arc;

use crate::common::network::replication::entity::{client, server};

pub struct Identifier {
	pub client: Arc<client::AppContext>,
	pub server: Arc<server::AppContext>,
}

impl stream::Identifier for Identifier {
	type SendBuilder = server::AppContext;
	type RecvBuilder = client::AppContext;
	fn unique_id() -> &'static str {
		"replication::entity"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.server
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.client
	}
}
