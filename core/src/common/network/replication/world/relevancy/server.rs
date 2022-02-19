use crate::common::network::replication::world::{RecvUpdate, SendChunks};
use crate::entity::system::replicator::relevancy;
use anyhow::Result;
use socknet::stream;
use socknet::{
	connection::Connection,
	stream::kind::{recv, send},
};
use std::sync::Arc;

/// The application context for the server/sender of a world-relevancy stream.
#[derive(Default)]
pub struct AppContext;

/// Opening the stream using an outgoing bidirectional stream
impl stream::send::AppContext for AppContext {
	type Opener = stream::bi::Opener;
}

/// The stream handler for the server/sender of a world-relevancy stream.
pub struct Sender {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: send::Ongoing,
	recv: recv::Ongoing,
}

impl From<stream::send::Context<AppContext>> for Sender {
	fn from(context: stream::send::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream.0,
			recv: context.stream.1,
		}
	}
}

impl stream::handler::Initiator for Sender {
	type Identifier = super::Identifier;
}

impl Sender {
	/// Ongoing async task which dispatches relevancy updates to the client.
	/// When each update is acknowledged, the relevant chunks are sent
	/// through the provided send channel to be replicated.
	pub async fn send_until_closed(
		&mut self,
		channel: RecvUpdate,
		send_chunks: SendChunks,
	) -> Result<()> {
		while let Ok(update) = channel.recv().await {
			match update {
				relevancy::WorldUpdate::Relevance(relevance) => {
					// We await on the relevance response before sending futher updates
					// (i.e. before we send the chunk messages to the chunk streams).
					self.send_relevance(relevance).await?;
				}
				relevancy::WorldUpdate::Chunks(chunks) => {
					for chunk in chunks.into_iter() {
						send_chunks.send(chunk).await?;
					}
				}
			}
		}
		Ok(())
	}

	/// Sends an individual relevance update, and awaits until it is acknowledged by the client.
	async fn send_relevance(&mut self, relevance: relevancy::Relevance) -> Result<()> {
		use stream::kind::{Read, Write};

		// Send a net relevancy notification
		self.send.write(&relevance).await?;

		// Wait for acknowledgement byte from client
		let _ = self.recv.read_size().await?;

		Ok(())
	}
}
