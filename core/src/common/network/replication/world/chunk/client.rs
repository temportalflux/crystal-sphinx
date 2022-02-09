use super::Builder;
use engine::{
	math::nalgebra::Point3,
	network::socknet::{
		connection::{self, Connection},
		stream::{self, kind::recv::Ongoing},
	},
};
use std::sync::Arc;

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
			let log = format!(
				"client/{}[{}]",
				Builder::unique_id(),
				self.connection.remote_address()
			);

			let coord = self.recv.read::<Point3<i64>>().await?;
			log::info!(
				target: &log,
				"Receiving data for chunk(<{}, {}, {}>)",
				coord.x,
				coord.y,
				coord.z
			);

			// TODO: Receive chunk data, confirm it is expected (otherwise discard it).
			// Mark as confirmed/acknowledged and update local data to match the chunk.

			Ok(())
		});
	}
}
