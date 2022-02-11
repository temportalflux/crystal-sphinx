use std::sync::Arc;

use crate::common::network::move_player::Datum;
use engine::utility::Result;
use socknet::{
	connection::Connection,
	stream::{
		self,
		kind::send::{self},
	},
};

use super::Builder;

type Context = stream::Context<Builder, send::Datagram>;

pub struct Sender {
	#[allow(dead_code)]
	context: Arc<Builder>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: send::Datagram,
}

impl From<Context> for Sender {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream,
		}
	}
}

impl stream::handler::Initiator for Sender {
	type Builder = Builder;
}

impl Sender {
	pub async fn initiate(&mut self) -> Result<()> {
		use stream::{kind::Write, Identifier};
		self.send.write(&Builder::unique_id().to_owned()).await?;
		Ok(())
	}

	pub async fn send_datum(&mut self, datum: Datum) -> Result<()> {
		use stream::kind::{Send, Write};
		self.send.write(&datum).await?;
		self.send.finish().await?;
		Ok(())
	}
}
