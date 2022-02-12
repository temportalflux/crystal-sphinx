use std::sync::Arc;

use crate::common::network::move_player::Datum;
use anyhow::Result;
use socknet::{
	connection::Connection,
	stream::{
		self,
		kind::send::{self},
	},
};

#[derive(Default)]
pub struct AppContext;

impl stream::send::AppContext for AppContext {
	type Opener = stream::datagram::Opener;
}

pub struct Sender {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: send::Datagram,
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
	pub async fn send_datum(&mut self, datum: Datum) -> Result<()> {
		use stream::kind::{Send, Write};
		self.send.write(&datum).await?;
		self.send.finish().await?;
		Ok(())
	}
}
