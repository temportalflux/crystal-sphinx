use crate::{
	common::network::replication::world::RecvChunks, server::world::chunk::Chunk as ServerChunk,
};
use anyhow::Result;
use socknet::{
	connection::Connection,
	stream::{self, kind::send::Ongoing},
};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct AppContext;

impl stream::send::AppContext for AppContext {
	type Opener = stream::uni::Opener;
}

pub struct Sender {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: Ongoing,
}

impl From<stream::send::Context<AppContext>> for Sender {
	fn from(context: stream::send::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream,
		}
	}
}

impl stream::handler::Initiator for Sender {
	type Identifier = super::Identifier;
}

impl Sender {
	pub async fn send_until_closed(&mut self, index: usize, recv_chunks: RecvChunks) -> Result<()> {
		use stream::kind::Write;
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
