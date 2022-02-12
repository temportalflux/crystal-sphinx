use crate::app;
use anyhow::Result;
use socknet::{self, connection::Connection, stream};
use std::sync::{Arc, RwLock, Weak};

pub struct AppContext {
	pub app_state: Weak<RwLock<app::state::Machine>>,
}

impl stream::send::AppContext for AppContext {
	type Opener = stream::bi::Opener;
}

pub struct Handshake {
	context: Arc<AppContext>,
	connection: Arc<Connection>,
	send: stream::kind::send::Ongoing,
	recv: stream::kind::recv::Ongoing,
}

impl From<stream::send::Context<AppContext>> for Handshake {
	fn from(context: stream::send::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream.0,
			recv: context.stream.1,
		}
	}
}

impl stream::handler::Initiator for Handshake {
	type Identifier = super::Identifier;
}

impl Handshake {
	fn app_state(&self) -> Result<Arc<RwLock<app::state::Machine>>> {
		Ok(self
			.context
			.app_state
			.upgrade()
			.ok_or(Error::InvalidAppState)?)
	}

	pub fn initiate(mut self) {
		self.connection.clone().spawn(async move {
			use stream::Identifier;
			let log = super::Identifier::log_category("client", &self.connection);
			self.process(&log).await?;
			Ok(())
		});
	}

	async fn process(&mut self, log: &str) -> Result<()> {
		use anyhow::Context;
		use stream::kind::{Read, Write};
		log::info!(target: &log, "Initiating handshake");

		let display_name = {
			use crate::client::account;
			let registry = account::Manager::read().unwrap();
			let account = registry
				.active_account()
				.context("send account data to server")?;
			account.display_name().clone()
		};

		let key_pair = {
			use ring::signature::{self, EcdsaKeyPair};
			let source = self.connection.endpoint()?;
			let private_key = source.private_key();
			EcdsaKeyPair::from_pkcs8(&signature::ECDSA_P256_SHA256_ASN1_SIGNING, &private_key.0)
				.map_err(|err| Error::KeyRejected(err.description_()))?
		};

		// Step 1: Send the client's public key
		{
			use ring::signature::KeyPair;
			self.send
				.write_bytes(key_pair.public_key().as_ref())
				.await
				.context("writing public key")?;
		}

		// Step 2: Disconnected if our account has joined before and had a different public key.

		// Tell the server who we think we are.
		self.send
			.write(&display_name)
			.await
			.context("writing display name")?;

		// Step 3: Sign the random token & send it to the server.
		let token = self.recv.read_bytes().await.context("reading token")?;
		let signature = {
			use ring::rand::SystemRandom;

			let rng = SystemRandom::new();
			let signature = key_pair
				.sign(&rng, &token)
				.map_err(|_| Error::FailedToSignToken)?;

			signature
		};
		self.send
			.write_bytes(&signature.as_ref())
			.await
			.context("writing token")?;

		// Step 4: Receive an approval byte if we've been authenticated.
		let authenticated = self.recv.read::<bool>().await?;

		// Streams are going to be stopped regardless.
		// If we have failed auth, the connection will also be closed.

		// TODO: Server should send a "all data is ready" signal to tell the client
		// that it is safe to enter the game, once relevant chunks and entities have been loaded.
		// Must require:
		// - player's entity and components have been replicated

		self.app_state()?.write().unwrap().transition_to(
			match authenticated {
				true => crate::app::state::State::InGame,
				false => crate::app::state::State::MainMenu,
			},
			None,
		);

		Ok(())
	}
}

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error("Key rejected during parsing: {0}")]
	KeyRejected(&'static str),
	#[error("Failed to sign handshake token")]
	FailedToSignToken,

	#[error("Application state machine is invalid")]
	InvalidAppState,
}
