use super::Builder;
use engine::{
	math::nalgebra::Point3,
	network::socknet::{
		connection::{self, Connection},
		stream::{self, kind::send::Ongoing},
	},
	utility::Result,
};
use std::{net::SocketAddr, sync::Arc};

pub type Context = stream::Context<Builder, Ongoing>;

pub struct Chunk {
	#[allow(dead_code)]
	context: Arc<Builder>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: Ongoing,
}

impl From<Context> for Chunk {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream,
		}
	}
}

impl stream::handler::Initiator for Chunk {
	type Builder = Builder;
}

impl Chunk {
	fn log_target(kind: &str, address: &SocketAddr) -> String {
		use stream::Identifier;
		format!("{}/{}[{}]", kind, Builder::unique_id(), address)
	}

	pub async fn initiate(&mut self) -> Result<()> {
		use connection::Active;
		use stream::{kind::Write, Identifier};
		let log = Self::log_target("server", &self.connection.remote_address());
		log::debug!(target: &log, "Establishing stream");
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(())
	}

	pub async fn write_chunk(&mut self, coord: Point3<i64>) -> Result<()> {
		use stream::kind::Write;
		self.send.write(&coord).await?;
		// TODO: Use the server world database to write data about the chunk to the stream
		Ok(())
	}
}
