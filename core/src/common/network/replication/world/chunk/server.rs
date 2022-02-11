use super::Builder;
use crate::server::world::chunk::Chunk as ServerChunk;
use engine::{
	math::nalgebra::Point3,
	network::socknet::{
		connection::{self, Connection},
		stream::{self, kind::send::Ongoing},
	},
	utility::Result,
};
use std::{
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

pub type Context = stream::Context<Builder, Ongoing>;
pub type RecvChunks = async_channel::Receiver<Weak<RwLock<ServerChunk>>>;

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
	fn log_target(kind: &str, address: &SocketAddr, index: usize) -> String {
		use stream::Identifier;
		format!("{}/{}[{}][{}]", kind, Builder::unique_id(), address, index)
	}

	pub fn spawn(connection: Weak<Connection>, index: usize, recv_chunks: RecvChunks) {
		let arc = Connection::upgrade(&connection).unwrap();
		arc.spawn(async move {
			use connection::Active;
			use stream::handler::Initiator;
			let mut stream = Self::open(&connection)?.await?;
			stream.initiate().await?;
			stream.send_until_closed(index, recv_chunks).await?;
			Ok(())
		});
	}

	async fn initiate(&mut self) -> Result<()> {
		use stream::{kind::Write, Identifier};
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(())
	}

	async fn send_until_closed(&mut self, index: usize, recv_chunks: RecvChunks) -> Result<()> {
		use connection::Active;
		use stream::{kind::Write, Identifier};
		self.send.write_size(index).await?;
		while let Ok(weak_server_chunk) = recv_chunks.recv().await {
			let arc_server_chunk = match weak_server_chunk.upgrade() {
				Some(arc) => arc,
				// If the chunk has been unloaded, then we dont need to replicated it.
				None => return Ok(()),
			};
			self.write_chunk(arc_server_chunk).await?;
		}
		Ok(())
	}

	pub async fn write_chunk(&mut self, arc_server_chunk: Arc<RwLock<ServerChunk>>) -> Result<()> {
		use stream::kind::Write;
		let chunk = {
			let server_chunk = arc_server_chunk.read().unwrap();
			server_chunk.chunk.clone()
		};

		self.send.write(&chunk.coordinate).await?;

		self.send.write_size(chunk.block_ids.len()).await?;

		for (offset, block_id) in chunk.block_ids.into_iter() {
			let offset = offset.cast::<u8>();
			self.send.write(&offset).await?;
			self.send.write(&block_id).await?;
		}

		Ok(())
	}
}
