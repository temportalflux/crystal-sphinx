use crate::{
	common::{
		account,
		network::{client_joined, connection, mode, Broadcast, CloseCode, Storage},
	},
	entity,
	server::network::Storage as ServerStorage,
};
use anyhow::Result;
use socknet::{self, connection::Connection, stream};
use std::sync::{Arc, RwLock, Weak};

pub struct AppContext {
	pub storage: Weak<RwLock<Storage>>,
	pub entity_world: Weak<RwLock<entity::World>>,
}

impl stream::recv::AppContext for AppContext {
	type Extractor = stream::bi::Extractor;
	type Receiver = Handshake;
}

pub struct Handshake {
	context: Arc<AppContext>,
	connection: Arc<Connection>,
	send: stream::kind::send::Ongoing,
	recv: stream::kind::recv::Ongoing,
}

impl From<stream::recv::Context<AppContext>> for Handshake {
	fn from(context: stream::recv::Context<AppContext>) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			send: context.stream.0,
			recv: context.stream.1,
		}
	}
}

impl Handshake {
	fn storage(&self) -> Result<Arc<RwLock<Storage>>> {
		use crate::common::network::Error::InvalidStorage;
		Ok(self.context.storage.upgrade().ok_or(InvalidStorage)?)
	}

	fn server(&self) -> Result<Arc<RwLock<ServerStorage>>> {
		use crate::common::network::Error::{FailedToReadStorage, InvalidServer};
		let arc = self.storage()?;
		let storage = arc.read().map_err(|_| FailedToReadStorage)?;
		let server = storage.server().as_ref().ok_or(InvalidServer)?;
		Ok(server.clone())
	}

	fn connection_list(&self) -> Result<Arc<RwLock<connection::List>>> {
		use crate::common::network::Error::FailedToReadStorage;
		let arc = self.storage()?;
		let storage = arc.read().map_err(|_| FailedToReadStorage)?;
		Ok(storage.connection_list().clone())
	}

	fn entity_world(&self) -> Result<Arc<RwLock<entity::World>>> {
		Ok(self
			.context
			.entity_world
			.upgrade()
			.ok_or(Error::InvalidEntityWorld)?)
	}
}

impl stream::handler::Receiver for Handshake {
	type Identifier = super::Identifier;
	fn receive(mut self) {
		use stream::Identifier;
		let log = super::Identifier::log_category("server", &self.connection);
		self.connection.clone().spawn(log.clone(), async move {
			use anyhow::Context;
			use stream::kind::{Recv, Send};
			if let Err(error) = self
				.process_server(&log)
				.await
				.context("Failed authentication")
			{
				use socknet::connection::Active;
				log::error!(target: &log, "{:?}", error);
				self.recv.stop().await?;
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
		use crate::common::network::Error::{FailedToReadServer, FailedToWriteServer};
		use account::key::{Key, PublicKey};
		use anyhow::Context;
		use socknet::connection::Active;
		use stream::kind::{Read, Recv, Send, Write};

		let account_id = self.connection.fingerprint()?;
		log::info!(
			target: &log,
			"Received handshake from account({})",
			account_id
		);

		// Step 1: Receive the client's public key
		// (which is derived from there private_key and is different from the certificate)
		let public_key = self.recv.read_bytes().await.context("reading public key")?;
		let public_key = PublicKey::from_bytes(public_key);
		log::info!(target: &log, "Received {}", public_key);

		let (arc_user, is_new) = {
			let server = self.server().context("fetching server data")?;
			let server = server
				.read()
				.map_err(|_| FailedToReadServer)
				.context("finding user")?;
			match server.find_user(&account_id) {
				Some(arc_user) => (arc_user.clone(), false),
				None => {
					use crate::server::user;
					use account::Account;
					let account = Account::new_public(
						&server.get_players_dir_path(),
						account_id.clone(),
						public_key.clone(),
					);
					let arc_user = Arc::new(RwLock::new(user::Active::new(account)));
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

		self.send.write(&verified).await?;

		self.recv.stop().await?;
		self.send.finish().await?;

		if !verified {
			log::info!(target: &log, "Failed authentication");
			self.connection
				.close(CloseCode::FailedAuthentication as u32, &vec![]);
			return Ok(());
		}

		log::info!(target: &log, "Passed authentication");

		if is_new {
			let server = self.server().context("fetching server data")?;
			let mut server = server
				.write()
				.map_err(|_| FailedToWriteServer)
				.context("adding user")?;
			server.add_user(account_id.clone(), arc_user);
		}

		// Broadcast authenticated event locally to initiate other objects (like replication streams)
		let connection_list = self.connection_list()?;
		connection_list
			.write()
			.map_err(|_| connection::Error::FailedToWriteList)?
			.broadcast(connection::Event::Authenticated(
				self.connection.remote_address(),
				Arc::downgrade(&self.connection),
			));

		{
			use entity::archetype;
			let arc_world = self.entity_world()?;
			let mut world = arc_world.write().unwrap();
			log::debug!(
				target: &log,
				"Initializing entity for new player({})",
				account_id
			);

			// Build an entity for the player which is marked with
			// the account id of the user and the ip address of the connection.
			let mut builder = archetype::player::Server::new()
				.with_user_id(account_id.clone())
				.with_address(self.connection.remote_address())
				.build();

			// Integrated Client-Server needs to spawn client-only components
			// if its the local player's entity.
			if mode::get().contains(mode::Kind::Client) {
				let client_reg = crate::client::account::Manager::read().unwrap();
				let local_account = client_reg.active_account().unwrap();
				// If the account ids match, then this entity is the local player's avatar
				if *local_account.id() == *account_id {
					builder = archetype::player::Client::apply_to(builder);
				}
			}

			world.spawn(builder.build());
		}

		Broadcast::<client_joined::Sender>::new(connection_list)
			.with_on_established(move |client_joined: client_joined::Sender| {
				let account_id = account_id.clone();
				Box::pin(async move {
					client_joined.send(account_id).await?;
					Ok(())
				})
			})
			.open();

		Ok(())
	}
}

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error("failed to read user for id({0})")]
	FailedToReadUser(String),
	#[error("provided public key did not match previous login")]
	InvalidPublicKey,

	#[error("Entity World is invalid")]
	InvalidEntityWorld,
}
