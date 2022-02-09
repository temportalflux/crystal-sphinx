use crate::block;

use super::Builder;
use engine::{
	math::nalgebra::Point3,
	network::socknet::{
		connection::{self, Connection},
		stream::{self, kind::recv::Ongoing},
	},
};
use std::{sync::Arc, time::Instant};

pub type Context = stream::Context<Builder, Ongoing>;

pub struct Chunk {
	#[allow(dead_code)]
	context: Arc<Builder>,
	connection: Arc<Connection>,
	recv: Ongoing,
}

impl From<Context> for Chunk {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			recv: context.stream,
		}
	}
}

impl stream::handler::Receiver for Chunk {
	type Builder = Builder;
	fn receive(mut self) {
		self.connection.clone().spawn(async move {
			use connection::Active;
			use stream::{kind::Read, Identifier};

			let start_time = Instant::now();

			let coord = self.recv.read::<Point3<i64>>().await?;
			let log = format!(
				"client/{}[{}]<{}, {}, {}>",
				Builder::unique_id(),
				self.connection.remote_address(),
				coord.x,
				coord.y,
				coord.z
			);

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
						"Chunk is being discarded because it is no longer relevant to {:?}.",
						relevance
					);
					return Ok(());
				}
			}

			if let Ok(mut cache) = self.context.client_chunk_cache()?.write() {
				cache.insert_updates(&coord, &contents);
			}

			Ok(())
		});
	}
}
