use crate::{
	client::world::chunk, common::network::Storage, entity::system::replicator::relevancy,
};
use anyhow::Result;
use socknet::stream;
use socknet::{
	connection::Connection,
	stream::kind::{recv, send},
};
use std::sync::{Arc, RwLock, Weak};

#[derive(Default)]
pub struct AppContext {
	pub storage: Weak<RwLock<Storage>>,
	pub local_relevance: Arc<RwLock<relevancy::Relevance>>,
}

/// Receiving the handler results in an incoming bidirectional stream
impl stream::recv::AppContext for AppContext {
	type Extractor = stream::bi::Extractor;
	type Receiver = Handler;
}

impl AppContext {
	fn client_chunk_sender(&self) -> Result<chunk::OperationSender> {
		use crate::common::network::Error::{
			FailedToReadClient, FailedToReadStorage, InvalidClient, InvalidStorage,
		};
		let arc_storage = self.storage.upgrade().ok_or(InvalidStorage)?;
		let storage = arc_storage.read().map_err(|_| FailedToReadStorage)?;
		let arc = storage.client().as_ref().ok_or(InvalidClient)?;
		let client = arc.read().map_err(|_| FailedToReadClient)?;
		Ok(client.chunk_sender().clone())
	}
}

pub struct Handler {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	connection: Arc<Connection>,
	send: send::Ongoing,
	recv: recv::Ongoing,
}

impl From<stream::recv::Context<AppContext>> for Handler {
	fn from(context: stream::recv::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream.0,
			recv: context.stream.1,
		}
	}
}

impl stream::handler::Receiver for Handler {
	type Identifier = super::Identifier;
	fn receive(mut self) {
		use stream::Identifier;
		let log = super::Identifier::log_category("client", &self.connection);
		self.connection.clone().spawn(log.clone(), async move {
			use stream::kind::{Read, Write};

			// Read any incoming relevancy until the client is disconnected.
			while let Ok(relevance) = self.recv.read::<relevancy::Relevance>().await {
				// Get the set of chunks which are only in the old relevance,
				// and write the new relevance to the shared list.
				let old_chunks = {
					// Contain the write-lock on local relevance to only this block
					// so it doesn't get held after the acknowledgement is sent.
					let mut local_relevance = self.context.local_relevance.write().unwrap();
					// Compare old relevance with new relevance to determine what chunks are no longer relevant
					let old_chunks = local_relevance.difference(&relevance);
					// Save new relevance (before sending acknowledgement) so that the incoming chunk packets are actually processed
					*local_relevance = relevance.clone();
					old_chunks
				};

				// Acknowledge that the relevancy was received and we are
				// ready to receive the individual streams for chunk data.
				self.send.write_size(0).await?;

				let mut old_chunks = old_chunks.into_iter().collect::<Vec<_>>();
				relevance.sort_vec_by_sig_dist(&mut old_chunks);

				// We can expect that sometime after the acknowledgement is sent,
				// the server will open streams for any/all new chunks to be replicated.
				// So its possible that those streams are now active while we are also
				// removing old chunks from the cache.
				if let Ok(sender) = self.context.client_chunk_sender() {
					for coord in old_chunks.into_iter().rev() {
						sender.try_send(chunk::Operation::Remove(coord))?;
					}
				}
			}

			// If relevancy has been dropped, then the client is expected to have been disconnected (voluntarily or otherwise).
			// We should clear the local relevancy to ensure that if the client joins another world, its already in the default state.
			log::debug!(target: &log, "Stream ended, clearing state.");
			if let Ok(mut local) = self.context.local_relevance.write() {
				*local = relevancy::Relevance::default();
			}

			Ok(())
		});
	}
}
