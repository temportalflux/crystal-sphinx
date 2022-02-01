use crate::common::network::replication::entity::Builder;
use engine::network::socknet::{
	connection::{self, Connection},
	stream::{self, kind::recv::Ongoing},
};
use std::sync::Arc;

pub type Context = stream::Context<Builder, Ongoing>;

pub struct Handler {
	#[allow(dead_code)]
	context: Arc<Builder>,
	connection: Arc<Connection>,
	recv: Ongoing,
}

impl From<Context> for Handler {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			recv: context.stream,
		}
	}
}

impl stream::handler::Receiver for Handler {
	type Builder = Builder;
	fn receive(mut self) {
		use connection::Active;
		use stream::Identifier;
		let log = format!(
			"client/{}[{}]",
			Builder::unique_id(),
			self.connection.remote_address()
		);
		engine::task::spawn(log.clone(), async move {
			log::info!(target: &log, "Stream opened");
			Ok(())
		});
	}
}
