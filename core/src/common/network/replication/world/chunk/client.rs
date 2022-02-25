use crate::{
	block, client::world::chunk, common::network::Storage,
	entity::system::replicator::relevancy::Relevance,
};

use engine::math::nalgebra::Point3;
use socknet::{
	connection::Connection,
	stream::{self, kind::recv::Ongoing},
};
use std::{
	sync::{Arc, RwLock, Weak},
	time::Instant,
};

/// The application context for the client/receiver of a chunk replication stream.
pub struct AppContext {
	pub local_relevance: Arc<RwLock<Relevance>>,
	pub storage: Weak<RwLock<Storage>>,
}

/// Creates the handler from an incoming unidirectional stream
impl stream::recv::AppContext for AppContext {
	type Extractor = stream::uni::Extractor;
	type Receiver = Handler;
}

impl AppContext {
	/// Returns the client application's chunk instance buffer operation sender
	/// (to send update operations to the graphics buffer).
	pub fn client_chunk_sender(&self) -> anyhow::Result<chunk::OperationSender> {
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

/// The stream handler for the client/receiver of a chunk replication stream.
pub struct Handler {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	connection: Arc<Connection>,
	recv: Ongoing,
}

impl From<stream::recv::Context<AppContext>> for Handler {
	fn from(context: stream::recv::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			recv: context.stream,
		}
	}
}

impl stream::handler::Receiver for Handler {
	type Identifier = super::Identifier;
	fn receive(mut self) {
		use stream::Identifier;
		let log = super::Identifier::log_category("client", &self.connection);
		self.connection.clone().spawn(log.clone(), async move {
			use stream::kind::Read;
			let index = self.recv.read_size().await?;
			while let Ok(coord) = self.recv.read::<Point3<i64>>().await {
				let log = format!("{}[{}]<{}, {}, {}>", log, index, coord.x, coord.y, coord.z);
				if let Err(err) = self.process_chunk(&log, coord).await {
					log::error!(target: &log, "{:?}", err);
				}
			}
			Ok(())
		});
	}
}

impl Handler {
	/// Reads a chunk from the stream, after the initial coordinate has been read.
	/// Keeps track of how long it took to replicate, and enqueues the new chunk for display once replication is complete.
	async fn process_chunk(&mut self, log: &str, coord: Point3<i64>) -> anyhow::Result<()> {
		use stream::kind::Read;
		let start_time = Instant::now();

		let block_count = self.recv.read_size().await?;
		let mut contents = Vec::with_capacity(block_count);
		for _ in 0..block_count {
			let offset = self.recv.read::<Point3<u8>>().await?;
			let offset = offset.cast::<usize>();
			let block_id = self.recv.read::<block::LookupId>().await?;
			contents.push((offset, block_id));
		}

		let end_time = Instant::now();
		let repl_duration = end_time.duration_since(start_time);

		if repl_duration.as_millis() > 2000 {
			log::warn!(
				target: &log,
				"Took {:.2}s ({}ms) to replicate.",
				repl_duration.as_secs_f32(),
				repl_duration.as_millis()
			);
		}

		// Ensure that the sent chunk is actually relevant.
		// While its not expected that the server sends no-longer relevant chunks,
		// it is plausible that a chunk was sent, but the client has since moved.
		// We /could/ have checked this as soon as we got the coord,
		// but its more likely the client moved out of range while all of the date was being received.
		if let Ok(relevance) = self.context.local_relevance.read() {
			if !relevance.is_relevant(&coord) {
				log::warn!(
					target: &log,
					"Chunk is being discarded because it is no longer relevant to {:?} (min-dist={:.2}).",
					relevance,
					relevance.min_dist_to_relevance(&coord),
				);
				return Ok(());
			}
		}

		self.context
			.client_chunk_sender()?
			.try_send(chunk::Operation::Insert(coord, contents))?;

		Ok(())
	}
}
