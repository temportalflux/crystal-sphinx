use crate::common::network::replication::entity::{Builder, Channel};
use engine::utility::Result;
use socknet::{
	connection::Connection,
	stream::{self, kind::send::Ongoing},
};
use std::sync::{Arc, Weak};

pub type Context = stream::Context<Builder, Ongoing>;

pub struct Handler {
	#[allow(dead_code)]
	context: Arc<Builder>,
	#[allow(dead_code)]
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
	pub fn spawn(connection: Weak<Connection>, channel: Channel) {
		let arc = Connection::upgrade(&connection).unwrap();
		arc.spawn(async move {
			use stream::handler::Initiator;
			let mut stream = Handler::open(&connection)?.await?;
			stream.initiate().await?;
			stream.send_until_closed(channel).await?;
			Ok(())
		});
	}

	async fn initiate(&mut self) -> Result<()> {
		use stream::{kind::Write, Identifier};
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(())
	}

	async fn send_until_closed(&mut self, channel: Channel) -> Result<()> {
		use stream::kind::Write;
		while let Ok(update) = channel.recv().await {
			self.send.write(&update).await?;
		}
		Ok(())
	}
}
