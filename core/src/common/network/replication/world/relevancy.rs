use crate::entity::system::replicator::relevancy;
use engine::{
	math::nalgebra::Point3,
	network::socknet::{
		connection::{self, Connection},
		stream::{
			self,
			kind::{recv, send, Bidirectional},
		},
	},
	utility::Result,
};
use serde::{Deserialize, Serialize};
use std::{
	net::SocketAddr,
	sync::{Arc, Weak}, collections::HashSet,
};

/// Builder context for entity replication stream
pub struct Builder {}

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

pub type Context = stream::Context<Builder, Bidirectional>;
pub type Channel = async_channel::Receiver<(relevancy::Relevance, Option<HashSet<Point3<i64>>>)>;

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
	pub fn spawn(connection: Weak<Connection>, channel: Channel) {
		let arc = Connection::upgrade(&connection).unwrap();
		arc.spawn(async move {
			use connection::Active;
			use stream::handler::Initiator;
			let mut stream = Self::open(&connection)?.await?;
			let log = Self::log_target("server", &stream.connection.remote_address());
			log::debug!(target: &log, "Establishing stream");
			stream.initiate().await?;
			stream.send_until_closed(&log, channel).await?;
			log::debug!(target: &log, "Closing stream");
			Ok(())
		});
	}

	async fn initiate(&mut self) -> Result<()> {
		use stream::{kind::Write, Identifier};
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(())
	}

	async fn send_until_closed(&mut self, log: &str, channel: Channel) -> Result<()> {
		log::debug!(target: &log, "send_until_closed");
		while let Ok((relevance, new_chunks)) = channel.recv().await {
			log::debug!(target: &log, "Preparing to send relevance");
			self.send_relevance(relevance).await?;
			if let Some(coords) = new_chunks {
				self.open_repl_streams(coords).await?;
			}
		}
		log::debug!(target: &log, "</>send_until_closed");
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

	async fn open_repl_streams(
		&self,
		coords: HashSet<Point3<i64>>,
	) -> Result<()> {
		// Spin up individual streams for each chunk now that the client is expecting them
		let weak_connection = Arc::downgrade(&self.connection);
		for chunk_coord in coords.into_iter() {
			let connection = weak_connection.clone();
			self.connection.spawn(async move {
				use super::chunk::server::Chunk;
				use stream::handler::Initiator;
				let mut stream = Chunk::open(&connection)?.await?;
				stream.initiate().await?;
				stream.write_chunk(chunk_coord).await?;
				Ok(())
			});
		}
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
			while let Ok(relevance) = self.recv.read::<relevancy::Relevance>().await {
				// TODO: update some local data so that we know what chunks we are expecting

				// Acknowledge that the relevancy was received and we are
				// ready to receive the individual streams for chunk data.
				self.send.write_size(0).await?;
			}
			Ok(())
		});
	}
}
