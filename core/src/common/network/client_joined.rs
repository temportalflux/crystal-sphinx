use crate::common::account;
use anyhow::Result;
use socknet::{
	connection::{self, Connection},
	stream,
};
use std::sync::Arc;

#[derive(Default)]
pub struct Identifier(Arc<AppContext>);
impl stream::Identifier for Identifier {
	type SendBuilder = AppContext;
	type RecvBuilder = AppContext;
	fn unique_id() -> &'static str {
		"client_joined"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.0
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.0
	}
}

#[derive(Default)]
pub struct AppContext;
impl stream::send::AppContext for AppContext {
	type Opener = stream::uni::Opener;
}
impl stream::recv::AppContext for AppContext {
	type Extractor = stream::uni::Extractor;
	type Receiver = Receiver;
}

pub struct Sender {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: stream::kind::send::Ongoing,
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
	type Identifier = Identifier;
}
impl Sender {
	pub async fn send(mut self, account_id: account::Id) -> Result<()> {
		use stream::kind::{Send, Write};
		self.send.write(&account_id).await?;
		self.send.finish().await?;
		Ok(())
	}
}

pub struct Receiver {
	#[allow(dead_code)]
	context: Arc<AppContext>,
	connection: Arc<Connection>,
	recv: stream::kind::recv::Ongoing,
}
impl From<stream::recv::Context<AppContext>> for Receiver {
	fn from(context: stream::recv::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			recv: context.stream,
		}
	}
}
impl stream::handler::Receiver for Receiver {
	type Identifier = Identifier;
	fn receive(mut self) {
		use connection::Active;
		let log = format!(
			"{}[{}]",
			<Identifier as stream::Identifier>::unique_id(),
			self.connection.remote_address()
		);
		self.connection.clone().spawn(async move {
			use stream::kind::Read;
			let account_id = self.recv.read::<account::Id>().await?;
			log::info!(target: &log, "ClientJoined({})", account_id);
			// TODO: If some other client has authed, add their account::Meta to some known-clients list for display in a "connected users" ui
			Ok(())
		});
	}
}
