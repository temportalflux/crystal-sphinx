use crate::{
	common::{
		account,
		network::{CloseCode, ConnectionList},
	},
	network::storage::{server::ArcLockServer, Storage},
};
use engine::{
	network::socknet::{connection::Connection, stream},
	utility::{self, Result},
};
use std::sync::{Arc, RwLock, Weak};

pub type Context = stream::Context<Builder, stream::kind::Bidirectional>;
pub struct Builder {
	storage: Weak<RwLock<Storage>>,
}
impl stream::Identifier for Builder {
	fn unique_id() -> &'static str {
		"handshake"
	}
}
impl stream::send::Builder for Builder {
	type Opener = stream::bi::Opener;
}
impl stream::recv::Builder for Builder {
	type Extractor = stream::bi::Extractor;
	type Receiver = Handshake;
}

pub struct Handshake {
	context: Arc<Builder>,
	connection: Arc<Connection>,
	send: stream::kind::Send,
	recv: stream::kind::Recv,
}

impl From<Context> for Handshake {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream.0,
			recv: context.stream.1,
		}
	}
}

impl Handshake {
	pub fn builder(storage: Weak<RwLock<Storage>>) -> Builder {
		Builder { storage }
	}

	fn log(&self, side: &str) -> String {
		use stream::Identifier;
		format!(
			"{}/{}[{}]",
			side,
			Builder::unique_id(),
			self.connection.remote_address()
		)
	}

	fn storage(&self) -> Result<Arc<RwLock<Storage>>> {
		Ok(self
			.context
			.storage
			.upgrade()
			.ok_or(Error::InvalidStorage)?)
	}

	fn server(&self) -> Result<ArcLockServer> {
		let arc = self.storage()?;
		let storage = arc.read().map_err(|_| Error::FailedToReadStorage)?;
		let server = storage.server().as_ref().ok_or(Error::InvalidServer)?;
		Ok(server.clone())
	}

	fn connection_list(&self) -> Result<Arc<RwLock<ConnectionList>>> {
		let arc = self.storage()?;
		let storage = arc.read().map_err(|_| Error::FailedToReadStorage)?;
		Ok(storage.connection_list().clone())
	}
}

static PASSED_AUTH: u32 = 0;
static FAILED_AUTH: u32 = 1;

impl stream::handler::Initiator for Handshake {
	type Builder = Builder;
}

impl Handshake {
	pub fn initiate(mut self) {
		let log = self.log("client");
		engine::task::spawn(log.clone(), async move {
			self.process_client(&log).await?;
			Ok(())
		});
	}

	async fn process_client(&mut self, log: &str) -> Result<()> {
		use stream::{
			kind::{Read, Write},
			Identifier,
		};
		use utility::Context;
		log::info!(target: &log, "Initiating handshake");

		let display_name = {
			use crate::client::account;
			let registry = account::Manager::read().unwrap();
			let account = registry
				.active_account()
				.context("send account data to server")?;
			account.display_name().clone()
		};

		// Tells the server how to process the stream (and establishes the stream).
		self.send
			.write(&Builder::unique_id().to_owned())
			.await
			.context("writing handshake id")?;

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
		let code = self
			.send
			.stopped()
			.await
			.context("waiting for end-of-stream")?;
		assert!(code == PASSED_AUTH || code == FAILED_AUTH);
		let _authenticated = code == PASSED_AUTH;

		// Streams are going to be stopped regardless.
		// If we have failed auth, the connection will also be closed.

		// TODO: If authenticated, move to InGame state
		// TODO: If authentication failed in any way, go back to MainMenu state

		Ok(())
	}
}

impl stream::handler::Receiver for Handshake {
	type Builder = Builder;
	fn receive(mut self) {
		let log = self.log("server");
		engine::task::spawn(log.clone(), async move {
			use stream::kind::Write;
			use utility::Context;
			if let Err(error) = self
				.process_server(&log)
				.await
				.context("Failed authentication")
			{
				log::error!(target: &log, "{:?}", error);
				self.recv.stop(FAILED_AUTH).await?;
				self.send.finish().await?;
				self.connection
					.close(CloseCode::FailedAuthentication as u32, &vec![0u8]);
			}
			Ok(())
		});
	}
}

impl Handshake {
	async fn process_server(&mut self, log: &String) -> Result<()> {
		use account::key::{Key, PublicKey};
		use stream::kind::{Read, Write};
		use utility::Context;
		log::info!(target: &log, "Received handshake");

		let account_id = self.connection.fingerprint()?;

		// Step 1: Receive the client's public key
		// (which is derived from there private_key and is different from the certificate)
		let public_key = self.recv.read_bytes().await.context("reading public key")?;
		let public_key = PublicKey::from_bytes(public_key);
		log::info!(target: &log, "Received {}", public_key);

		let (arc_user, is_new) = {
			let server = self.server().context("fetching server data")?;
			let server = server
				.read()
				.map_err(|_| Error::FailedToReadServer)
				.context("finding user")?;
			match server.find_user(&account_id) {
				Some(arc_user) => (arc_user.clone(), false),
				None => {
					use crate::server::user::User;
					use account::Account;
					let account = Account::new_public(
						&server.get_players_dir_path(),
						account_id.clone(),
						public_key.clone(),
					);
					let arc_user = Arc::new(RwLock::new(User::new(account)));
					(arc_user, true)
				}
			}
		};

		// Step 2: Determine if the account has joined before
		// TODO: If the account (whose id is the certificate's fingerprint) has never joined before,
		// then they automatically pass the first phase.
		// Otherwise, the client-provided public key must match the public key stored to file.
		// To store to file: base64 encode the bytes of the client-provided public key.
		if !is_new {
			let user = arc_user
				.read()
				.map_err(|_| Error::FailedToReadUser(account_id.clone()))
				.context("public key validation")?;
			if let Key::Public(account_key) = user.account().key() {
				if public_key != *account_key {
					return Err(Error::InvalidPublicKey)?;
				}
			} else {
				unimplemented!();
			}
		}

		let display_name = self
			.recv
			.read::<String>()
			.await
			.context("reading display name")?;
		{
			let mut user = arc_user.write().unwrap();
			user.account_mut().set_display_name(display_name);
		}

		// Step 3: Generate a random token and send it to be signed by the client
		let token = {
			use rand::Rng;
			let raw_token: String = rand::thread_rng()
				.sample_iter(&rand::distributions::Alphanumeric)
				.take(64)
				.map(char::from)
				.collect();
			bincode::serialize(&raw_token)?
		};
		self.send
			.write_bytes(&token)
			.await
			.context("sending token")?;

		// Step 4: Verify the signed token
		let signed_token = self.recv.read_bytes().await.context("reading token")?;

		let verified = {
			use ring::signature::{self, UnparsedPublicKey};
			let bytes = public_key.as_bytes()?;
			let key = UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &bytes);
			key.verify(&token, &signed_token).is_ok()
		};

		if !verified {
			log::info!(target: &log, "Failed authentication");
			self.recv.stop(FAILED_AUTH).await?;
			self.send.finish().await?;

			self.connection
				.close(CloseCode::FailedAuthentication as u32, &vec![]);
			return Ok(());
		}

		log::info!(target: &log, "Passed authentication");
		self.recv.stop(PASSED_AUTH).await?;
		self.send.finish().await?;

		if is_new {
			let server = self.server().context("fetching server data")?;
			let mut server = server
				.write()
				.map_err(|_| Error::FailedToWriteServer)
				.context("adding user")?;
			server.add_user(account_id, arc_user);
		}

		// TODO: spawn the user entity in the world

		// TODO: Broadcast to all clients that a user has connected
		let _ = self.connection_list()?;

		// TODO: Add the user to the active cache

		Ok(())
	}
}

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error("Key rejected during parsing: {0}")]
	KeyRejected(&'static str),
	#[error("Failed to sign handshake token")]
	FailedToSignToken,

	#[error("storage is invalid")]
	InvalidStorage,
	#[error("failed to read from storage data")]
	FailedToReadStorage,

	#[error("server storage is invalid")]
	InvalidServer,
	#[error("failed to read from server data")]
	FailedToReadServer,
	#[error("failed to write to server data")]
	FailedToWriteServer,

	#[error("failed to read user for id({0})")]
	FailedToReadUser(String),
	#[error("provided public key did not match previous login")]
	InvalidPublicKey,
}
