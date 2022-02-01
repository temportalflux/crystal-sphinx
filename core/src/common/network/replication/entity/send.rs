use crate::common::network::replication::entity::Builder;
use engine::{
	network::socknet::{
		connection::{self, Connection},
		stream::{self, kind::send::Ongoing},
	},
	utility::Result,
};
use std::{net::SocketAddr, sync::Arc};

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
	pub fn log_target(address: &SocketAddr) -> String {
		use stream::Identifier;
		format!("server/{}[{}]", Builder::unique_id(), address)
	}

	pub async fn initiate(mut self) -> Result<Self> {
		use connection::Active;
		use stream::{kind::Write, Identifier};
		let log = Self::log_target(&self.connection.remote_address());
		log::debug!(target: &log, "Establishing entity replication stream");
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(self)
	}
}
