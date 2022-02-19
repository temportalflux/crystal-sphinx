use anyhow::Result;
use socknet::{
	connection::Connection,
	stream::{self, kind::send::Ongoing},
};
use std::sync::Arc;

use crate::common::network::replication::entity::RecvUpdate;

/// The application context for the server/sender of the entity replication stream.
#[derive(Default)]
pub struct AppContext;

/// Opening the stream using an outgoing unidirectional stream
impl stream::send::AppContext for AppContext {
	type Opener = stream::uni::Opener;
}

/// The stream handler for the server/sender of the entity replication stream.
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
	/// Ongoing async task which dispatches the entity updates to the client.
	/// Will keep the stream alive until its connection or the provided channel closes.
	pub async fn send_until_closed(&mut self, channel: RecvUpdate) -> Result<()> {
		use stream::kind::Write;
		while let Ok(update) = channel.recv().await {
			self.send.write(&update).await?;
		}
		Ok(())
	}
}
