use crate::{
	block,
	common::{network::Storage, world::Database},
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
	pub database: Weak<RwLock<Database>>,
}

/// Creates the handler from an incoming unidirectional stream
impl stream::recv::AppContext for AppContext {
	type Extractor = stream::uni::Extractor;
	type Receiver = Handler;
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
		//let mut contents = Vec::with_capacity(block_count);
		let mut chunk = crate::common::world::chunk::Chunk::new(coord);
		for _ in 0..block_count {
			let offset = self.recv.read::<Point3<u8>>().await?;
			let offset = offset.cast::<usize>();
			let block_id = self.recv.read::<block::LookupId>().await?;
			//contents.push((offset, block_id));
			chunk.set_block_id(offset, Some(block_id));
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

		if let Some(database) = self.context.database.upgrade() {
			database
				.write()
				.unwrap()
				.insert_chunk(*chunk.coordinate(), Arc::new(RwLock::new(chunk)));
		}

		Ok(())
	}
}
