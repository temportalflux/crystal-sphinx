use crate::common::account;
use engine::utility::Result;
use socknet::{
	connection::{self, Connection},
	stream,
};
use std::sync::Arc;

pub struct ClientJoined {}
impl stream::Identifier for ClientJoined {
	fn unique_id() -> &'static str {
		"client_joined"
	}
}
impl stream::send::Builder for ClientJoined {
	type Opener = stream::uni::Opener;
}
impl stream::recv::Builder for ClientJoined {
	type Extractor = stream::uni::Extractor;
	type Receiver = RecvClientJoined;
}

pub type SendContext = stream::Context<ClientJoined, stream::kind::send::Ongoing>;
pub struct SendClientJoined {
	#[allow(dead_code)]
	context: Arc<ClientJoined>,
	#[allow(dead_code)]
	connection: Arc<Connection>,
	send: stream::kind::send::Ongoing,
}
impl From<SendContext> for SendClientJoined {
	fn from(context: SendContext) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream,
		}
	}
}
impl stream::handler::Initiator for SendClientJoined {
	type Builder = ClientJoined;
}
impl SendClientJoined {
	pub async fn initiate(mut self, account_id: account::Id) -> Result<()> {
		use stream::{kind::Write, Identifier};
		self.send
			.write(&ClientJoined::unique_id().to_owned())
			.await?;
		self.send.write(&account_id).await?;
		Ok(())
	}
}

pub type RecvContext = stream::Context<ClientJoined, stream::kind::recv::Ongoing>;
pub struct RecvClientJoined {
	#[allow(dead_code)]
	context: Arc<ClientJoined>,
	connection: Arc<Connection>,
	recv: stream::kind::recv::Ongoing,
}
impl From<RecvContext> for RecvClientJoined {
	fn from(context: RecvContext) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			recv: context.stream,
		}
	}
}
impl stream::handler::Receiver for RecvClientJoined {
	type Builder = ClientJoined;
	fn receive(mut self) {
		use connection::Active;
		use stream::Identifier;
		let log = format!(
			"{}[{}]",
			ClientJoined::unique_id(),
			self.connection.remote_address()
		);
		engine::task::spawn(log.clone(), async move {
			use stream::kind::Read;
			let account_id = self.recv.read::<account::Id>().await?;
			log::info!(target: &log, "ClientJoined({})", account_id);
			// TODO: If some other client has authed, add their account::Meta to some known-clients list for display in a "connected users" ui
			Ok(())
		});
	}
}
