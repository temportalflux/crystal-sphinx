use crate::common::network::handshake::{client, server};
use socknet::{self, stream};
use std::sync::Arc;

pub struct Identifier {
	pub client: Arc<client::AppContext>,
	pub server: Arc<server::AppContext>,
}

impl stream::Identifier for Identifier {
	type SendBuilder = client::AppContext;
	type RecvBuilder = server::AppContext;
	fn unique_id() -> &'static str {
		"handshake"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.client
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.server
	}
}
