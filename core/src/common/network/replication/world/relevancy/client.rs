use crate::{
	client::world::chunk, common::network::Storage, entity::system::replicator::relevancy,
};
use engine::math::nalgebra::Point3;
use socknet::stream;
use socknet::{
	connection::Connection,
	stream::kind::{recv, send},
};
use std::collections::HashSet;
use std::sync::{Arc, RwLock, Weak};

/// The application context for the client/receiver of a world-relevancy stream.
pub struct AppContext {
	pub chunk_channel: Option<Weak<crate::client::world::ChunkChannel>>,
	/// The network storage for the client.
	pub storage: Weak<RwLock<Storage>>,
	/// The world relevancy last received from the server.
	/// Arc-locked so it can also be used by each [data stream](super::super::chunk).
	pub local_relevance: Arc<RwLock<relevancy::Relevance>>,
}

/// Creates the handler from an incoming bidirectional stream
impl stream::recv::AppContext for AppContext {
	type Extractor = stream::bi::Extractor;
	type Receiver = Handler;
}

/// The stream handler for the client/receiver of a world-relevancy stream.
pub struct Handler {
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

			let Some(chunk_channel) = self.context.chunk_channel.as_ref() else { return Ok(()); };

			// Read any incoming relevancy until the client is disconnected.
			while let Ok(relevance) = self.recv.read::<relevancy::Relevance>().await {
				// Get the set of chunks which are only in the old relevance,
				// and write the new relevance to the shared list.
				let old_chunk_cuboids = {
					// Contain the write-lock on local relevance to only this block
					// so it doesn't get held after the acknowledgement is sent.
					let mut local_relevance = self.context.local_relevance.write().unwrap();
					// Compare old relevance with new relevance to determine what chunks are no longer relevant
					let cuboids = local_relevance.difference(&relevance);
					// Save new relevance (before sending acknowledgement) so that the incoming chunk packets are actually processed
					*local_relevance = relevance.clone();
					cuboids
				};

				// Acknowledge that the relevancy was received and we are
				// ready to receive the individual streams for chunk data.
				self.send.write_size(0).await?;

				let mut old_chunks = Vec::with_capacity(old_chunk_cuboids.len());
				for cuboid in old_chunk_cuboids.into_iter() {
					let cuboid_coords: HashSet<Point3<i64>> = cuboid.into();
					for coord in cuboid_coords.into_iter() {
						old_chunks.push(coord);
					}
				}
				relevance.sort_vec_by_sig_dist(&mut old_chunks);

				// We can expect that sometime after the acknowledgement is sent,
				// the server will open streams for any/all new chunks to be replicated.
				// So its possible that those streams are now active while we are also
				// removing old chunks from the cache.
				let Some(chunk_channel) = chunk_channel.upgrade() else { continue; };
				for coord in old_chunks.into_iter().rev() {
					chunk_channel
						.send()
						.try_send(chunk::Operation::Remove(coord))?;
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
