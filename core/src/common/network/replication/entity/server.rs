use anyhow::Result;
use socknet::{
	connection::Connection,
	stream::{self, kind::send::Ongoing},
};
use std::sync::Arc;

use crate::common::network::replication::entity::update;

#[derive(Default)]
pub struct AppContext;
/// Opening the handler results in an outgoing unidirectional stream
impl stream::send::AppContext for AppContext {
	type Opener = stream::uni::Opener;
}

pub struct Sender {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: Ongoing,
}

impl From<stream::send::Context<AppContext>> for Sender {
	fn from(context: stream::send::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream,
		}
	}
}

impl stream::handler::Initiator for Sender {
	type Identifier = super::Identifier;
}

impl Sender {
	pub async fn send_until_closed(&mut self, channel: update::Receiver) -> Result<()> {
		use stream::kind::Write;
		while let Ok(update) = channel.recv().await {
			self.send.write(&update).await?;
		}
		Ok(())
	}
}
