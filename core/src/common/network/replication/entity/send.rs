use crate::common::network::replication::entity::{Builder, Channel};
use engine::{
	network::socknet::{
		connection::{self, Connection},
		stream::{self, kind::send::Ongoing},
	},
	utility::Result,
};
use std::{
	net::SocketAddr,
	sync::{Arc, Weak},
};

pub type Context = stream::Context<Builder, Ongoing>;

pub struct Handler {
	#[allow(dead_code)]
	context: Arc<Builder>,
	connection: Arc<Connection>,
	send: Ongoing,
}

impl From<Context> for Handler {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream,
		}
	}
}

impl stream::handler::Initiator for Handler {
	type Builder = Builder;
}

impl Handler {
	fn log_target(address: &SocketAddr) -> String {
		use stream::Identifier;
		format!("server/{}[{}]", Builder::unique_id(), address)
	}

	pub fn spawn(connection: Weak<Connection>, channel: Channel) {
		let arc = Connection::upgrade(&connection).unwrap();
		arc.spawn(async move {
			use connection::Active;
			use stream::handler::Initiator;
			let mut stream = Handler::open(&connection)?.await?;
			let log = Self::log_target(&stream.connection.remote_address());
			stream.initiate(&log).await?;
			stream.send_until_closed(&log, channel).await?;
			Ok(())
		});
	}

	async fn initiate(&mut self, log: &str) -> Result<()> {
		use stream::{kind::Write, Identifier};
		log::debug!(target: &log, "Establishing stream");
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(())
	}

	async fn send_until_closed(&mut self, log: &str, channel: Channel) -> Result<()> {
		use stream::kind::Write;
		while let Ok(update) = channel.recv().await {
			self.send.write(&update).await?;
		}
		Ok(())
	}
}
