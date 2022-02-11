use crate::{
	entity::system::replicator::relevancy, network::storage::Storage, server::world::chunk::Chunk,
};
use engine::{
	network::socknet::{
		connection::{self, Connection},
		stream::{
			self,
			kind::{recv, send, Bidirectional},
		},
	},
	utility::Result,
};
use std::{
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

/// Builder context for entity replication stream
pub struct Builder {
	pub local_relevance: Arc<RwLock<relevancy::Relevance>>,
	pub storage: Weak<RwLock<Storage>>,
}

/// The stream handler id is `replication::world`.
///
/// ```rust
/// use engine::network::socknet::stream::Identifier;
/// assert_eq!(Builder::unique::id(), "replication::world");
/// ```
impl stream::Identifier for Builder {
	fn unique_id() -> &'static str {
		"replication::world"
	}
}

/// Opening the handler results in an outgoing unidirectional stream
impl stream::send::Builder for Builder {
	type Opener = stream::bi::Opener;
}

/// Receiving the handler results in an incoming unidirectional stream
impl stream::recv::Builder for Builder {
	type Extractor = stream::bi::Extractor;
	type Receiver = Handler;
}

impl Builder {
	fn client_chunk_cache(&self) -> Result<crate::client::world::chunk::cache::ArcLock> {
		use crate::network::storage::Error::{
			FailedToReadClient, FailedToReadStorage, InvalidClient, InvalidStorage,
		};
		let arc_storage = self.storage.upgrade().ok_or(InvalidStorage)?;
		let storage = arc_storage.read().map_err(|_| FailedToReadStorage)?;
		let arc = storage.client().as_ref().ok_or(InvalidClient)?;
		let client = arc.read().map_err(|_| FailedToReadClient)?;
		Ok(client.chunk_cache().clone())
	}
}

pub type Context = stream::Context<Builder, Bidirectional>;
pub type RecvUpdate = async_channel::Receiver<relevancy::WorldUpdate>;
pub type SendChunks = async_channel::Sender<Weak<RwLock<Chunk>>>;

pub struct Handler {
	#[allow(dead_code)]
	context: Arc<Builder>,
	connection: Arc<Connection>,
	send: send::Ongoing,
	recv: recv::Ongoing,
}

impl From<Context> for Handler {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream.0,
			recv: context.stream.1,
		}
	}
}

impl Handler {
	fn log_target(kind: &str, address: &SocketAddr) -> String {
		use stream::Identifier;
		format!("{}/{}[{}]", kind, Builder::unique_id(), address)
	}
}

impl stream::handler::Initiator for Handler {
	type Builder = Builder;
}

impl Handler {
	pub fn spawn(connection: Weak<Connection>, channel: RecvUpdate, send_chunks: SendChunks) {
		let arc = Connection::upgrade(&connection).unwrap();
		arc.spawn(async move {
			use connection::Active;
			use stream::handler::Initiator;
			let mut stream = Self::open(&connection)?.await?;
			let log = Self::log_target("server", &stream.connection.remote_address());
			log::debug!(target: &log, "Establishing stream");
			stream.initiate().await?;
			stream.send_until_closed(channel, send_chunks).await?;
			log::debug!(target: &log, "Closing stream");
			Ok(())
		});
	}

	async fn initiate(&mut self) -> Result<()> {
		use stream::{kind::Write, Identifier};
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(())
	}

	async fn send_until_closed(
		&mut self,
		channel: RecvUpdate,
		send_chunks: SendChunks,
	) -> Result<()> {
		while let Ok(update) = channel.recv().await {
			match update {
				relevancy::WorldUpdate::Relevance(relevance) => {
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

	async fn send_relevance(&mut self, relevance: relevancy::Relevance) -> Result<()> {
		use stream::kind::{Read, Write};

		// Send a net relevancy notification
		self.send.write(&relevance).await?;

		// Wait for acknowledgement byte from client
		let _ = self.recv.read_size().await?;

		Ok(())
	}
}

impl stream::handler::Receiver for Handler {
	type Builder = Builder;
	fn receive(mut self) {
		self.connection.clone().spawn(async move {
			use connection::Active;
			use stream::kind::{Read, Write};

			let log = Self::log_target("client", &self.connection.remote_address());

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
				if let Ok(mut cache) = self.context.client_chunk_cache()?.write() {
					for coord in old_chunks.into_iter().rev() {
						cache.remove(&coord);
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
