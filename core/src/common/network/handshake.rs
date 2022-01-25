use engine::{
	network::socknet::{connection::Connection, initiator, responder, stream},
	utility::Result,
};
use std::sync::{Arc, Weak};

#[initiator(Bidirectional, process_client)]
#[responder("handshake", Bidirectional, process_server)]
pub struct Handshake {
	connection: Weak<Connection>,
	send: stream::Send,
	recv: stream::Recv,
}

impl Handshake {
	fn new(connection: Weak<Connection>, (send, recv): (stream::Send, stream::Recv)) -> Self {
		Self {
			connection,
			send,
			recv,
		}
	}

	fn connection(&self) -> Result<Arc<Connection>> {
		Connection::upgrade(&self.connection)
	}
}

impl Handshake {
	async fn process_client(mut self) -> Result<()> {
		static LOG: &'static str = "client::handshake";

		log::info!(
			target: LOG,
			"Initiating handshake to server({})",
			self.connection()?.fingerprint()?
		);

		self.send.write_id::<Handshake>().await?;

		let token = self.recv.read::<String>().await?;
		log::debug!(target: LOG, "Received token({})", token);

		let code = self.send.stopped().await?;
		assert_eq!(code, 0);

		Ok(())
	}

	async fn process_server(mut self) -> Result<()> {
		use rand::Rng;
		static LOG: &'static str = "server::handshake";

		log::info!(
			target: LOG,
			"Received handshake from client({})",
			self.connection()?.fingerprint()?
		);

		let token: String = rand::thread_rng()
			.sample_iter(&rand::distributions::Alphanumeric)
			.take(64)
			.map(char::from)
			.collect();
		log::debug!(target: LOG, "Sending token({})", token,);
		self.send.write(&token).await?;

		self.recv.stop(0).await?;
		self.send.finish().await?;

		Ok(())
	}
}
