use crate::{
	account,
	entity::{self, archetype, ArcLockEntityWorld},
	network::storage::{
		server::{user, ArcLockServer},
		ArcLockStorage,
	},
};
use engine::{
	network::{
		self,
		connection::Connection,
		enums::*,
		event, mode,
		packet::{Guarantee, Packet},
		packet_kind,
		processor::{AnyProcessor, EventProcessors, PacketProcessor, Processor},
		LocalData, Network, LOG,
	},
	utility::{AnyError, VoidResult},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock, Weak};

#[packet_kind(engine::network)]
#[derive(Serialize, Deserialize)]
pub struct Handshake(Request);

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Request {
	Login(account::Meta, /*public key*/ String),
	AuthTokenForClient(
		/*encrypted auth token*/ Vec<u8>,
		/*server public key*/ String,
	),
	AuthTokenForServer(/*re-encrypted auth token*/ Vec<u8>),
	ClientAuthenticated(account::Id),
}
impl std::fmt::Display for Request {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Login(account_meta, _client_key) => write!(f, "Login(id={})", account_meta.id),
			Self::AuthTokenForClient(_bytes, _server_key) => write!(f, "AuthTokenForClient"),
			Self::AuthTokenForServer(_bytes) => write!(f, "AuthTokenForServer"),
			Self::ClientAuthenticated(account_id) => {
				write!(f, "Authenticated(id={})", account_id)
			}
		}
	}
}

#[derive(Debug, Clone)]
enum Error {
	InvalidRequest(Request),
	CannotReadServerData,
	ClientKeyDoesntMatch(account::Id),
	ServerKeyCannotEncrypt,
	ClientTokenUnparsable,
	NoActiveAccount,
}
impl std::error::Error for Error {}
impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::InvalidRequest(request) => write!(f, "Invalid handshake request: {}", request),
			Self::CannotReadServerData => write!(f, "Cannot read from server data."),
			Self::ClientKeyDoesntMatch(account_id) => write!(f, "Client {} tried to authenticate, but their public key did not match a previous login.", account_id),
			Self::ServerKeyCannotEncrypt => write!(f, "Server's auth key could not become a public key... something blew up... X_X"),
			Self::ClientTokenUnparsable => write!(f, "Failed to parse decrypted token as a string."),
			Self::NoActiveAccount => write!(f, "Cannot authenticate client, no active account."),
		}
	}
}

impl Handshake {
	pub fn register(
		builder: &mut network::Builder,
		auth_cache: &user::pending::ArcLockCache,
		active_cache: &user::active::ArcLockCache,
		app_state: &crate::app::state::ArcLockMachine,
		storage: &ArcLockStorage,
		entity_world: &ArcLockEntityWorld,
	) {
		use mode::Kind::*;

		let server_proc = ServerProcessor {
			auth_cache: auth_cache.clone(),
			active_cache: active_cache.clone(),
			storage: storage.clone(),
			entity_world: Arc::downgrade(&entity_world),
		};
		let client_proc = ClientProcessor {
			app_state: app_state.clone(),
		};

		builder.register_bundle::<Handshake>(
			EventProcessors::default()
				.with(Server, server_proc.clone())
				.with(Client, client_proc.clone())
				.with(
					mode::Set::all(),
					AnyProcessor::new(vec![server_proc.boxed(), client_proc.boxed()]),
				),
		);
	}

	#[profiling::function]
	pub fn connect_to_server(address: &str) -> VoidResult {
		use network::prelude::*;
		let request = match account::ClientRegistry::read()?.active_account() {
			Some(account) => {
				Request::Login(account.meta.clone(), account.public_key().as_string()?)
			}
			None => return Ok(()),
		};
		Network::send_packets(
			Packet::builder()
				.with_address(address)?
				.with_guarantee(Reliable + Unordered)
				.with_payload(&Handshake(request)),
		)
	}
}

#[derive(Clone)]
struct ServerProcessor {
	auth_cache: user::pending::ArcLockCache,
	active_cache: user::active::ArcLockCache,
	storage: ArcLockStorage,
	entity_world: Weak<RwLock<entity::World>>,
}

impl ServerProcessor {
	fn server(&self) -> ArcLockServer {
		let storage = self.storage.read().unwrap();
		storage.server().as_ref().unwrap().clone()
	}
}

impl Processor for ServerProcessor {
	fn process(
		&self,
		kind: &event::Kind,
		data: &mut Option<event::Data>,
		local_data: &LocalData,
	) -> VoidResult {
		self.process_as(kind, data, local_data)
	}
}

impl PacketProcessor<Handshake> for ServerProcessor {
	#[profiling::function]
	fn process_packet(
		&self,
		_kind: &event::Kind,
		data: &mut Handshake,
		connection: &Connection,
		guarantee: &Guarantee,
		local_data: &LocalData,
	) -> VoidResult {
		match &data.0 {
			Request::Login(account_meta, public_key) => {
				let user_id = format!("{}({})", connection.address, account_meta.id);
				profiling::scope!("received-login-request", user_id.as_str());
				log::info!(target: LOG, "Received login request from {}", user_id);
				let (server_auth_key, user) = match self.server().read() {
					Ok(server) => (
						server.auth_key().clone(),
						server.find_user(&account_meta.id).cloned(),
					),
					Err(_) => {
						return Err(Box::new(Error::CannotReadServerData));
					}
				};

				// Auto-deny the logic in the public key stored locally doesnt match the provided one.
				if let Some(arclock_user) = user {
					if let Ok(saved_user_guard) = arclock_user.read() {
						if *public_key != saved_user_guard.public_key().as_string()? {
							// The server intentionally does not respond, which will cause the client to timeout.
							return Err(Box::new(Error::ClientKeyDoesntMatch(
								account_meta.id.clone(),
							)));
						}
					}
				}

				let token: String = rand::thread_rng()
					.sample_iter(&rand::distributions::Alphanumeric)
					.take(64)
					.map(char::from)
					.collect();
				log::debug!("Providing Token: {}", token);

				let client_public_key = account::Key::from_string(&public_key)?;
				let encrypted_bytes = if let account::Key::Public(rsa_public) = &client_public_key {
					use rand::rngs::OsRng;
					use rsa::PublicKey;
					let mut rng = OsRng;
					let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
					rsa_public.encrypt(&mut rng, padding, token.as_bytes())?
				} else {
					// This should never happen, because we requested the public key from the auth key.
					return Err(Box::new(Error::ServerKeyCannotEncrypt));
				};

				let mut pending_user = user::pending::User::new(
					connection.address,
					account_meta.clone(),
					client_public_key,
					token,
				);

				// NOTE: By sending this to the client, they will become connected (thats just how the network system works).
				// So if they fail auth, we need to kick them from the server.
				// If they disconnect (on their end or from getting kicked), we need to clear the pending request.
				let server_public_key = server_auth_key.public();
				data.0 =
					Request::AuthTokenForClient(encrypted_bytes, server_public_key.as_string()?);
				Network::send_packets(
					Packet::builder()
						.with_address(connection.address)?
						.with_guarantee(*guarantee)
						.with_payload(data),
				)?;

				// It is impossible to receive a response from the client since sending the packet
				// (execution of this process is blocking further packets being processsed),
				// so its safe to add to the cache after sending the packet.
				// We need to execute after packet send so that if there are any errors in sending the packet,
				// we dont have a lingering pending entry.
				if let Ok(mut auth_cache) = self.auth_cache.write() {
					pending_user.start_timeout(&self.auth_cache);
					auth_cache.insert(pending_user);
				}

				Ok(())
			}
			Request::AuthTokenForServer(reencrypted_token) => {
				let profiling_tag = format!("{}", connection.address);
				profiling::scope!("received-auth-token", profiling_tag.as_str());
				log::info!(
					target: LOG,
					"Received auth token from {}",
					connection.address
				);

				// Wrapper function to try to decrypt an auth token,
				// so that the error can be handled gracefully
				// without sacrificing readability.
				fn decrypt_token(bytes: &[u8], server: ArcLockServer) -> Result<String, AnyError> {
					let server_auth_key = match server.read() {
						Ok(server) => server.auth_key().clone(),
						Err(_) => {
							return Err(Box::new(Error::CannotReadServerData));
						}
					};

					match server_auth_key.decrypt(&bytes)? {
						Ok(token_bytes) => match String::from_utf8(token_bytes) {
							Ok(token) => return Ok(token),
							Err(_) => {
								return Err(Box::new(Error::ClientTokenUnparsable));
							}
						},
						Err(_) => {
							return Err(Box::new(Error::ClientTokenUnparsable));
						}
					}
				}

				// Extract the pending user struct from the auth cache,
				// or kick the user if none exits or the lock failed.
				let pending_user = match self.auth_cache.write() {
					// Remove the user from the cache (both on successful or failed auth cases).
					Ok(mut auth_cache) => match auth_cache.remove(&connection.address) {
						Some(pending_user) => pending_user,
						None => {
							// User may be missing if they've timed out.
							// They should be kicked.
							Network::kick(&connection.address)?;
							return Ok(());
						}
					},
					Err(_) => {
						Network::kick(&connection.address)?;
						return Ok(());
					}
				};

				// Decrypt the token bytes and try to process authentication,
				// handling the error case for a token beind undecryptable gracefully.
				match decrypt_token(&reencrypted_token, self.server()) {
					Ok(decrypted_token) => {
						// If the decrypted token does not match our records, they must be kicked.
						if decrypted_token != *pending_user.token() {
							log::warn!(
								target: LOG,
								"Pending user {}({}) has failed authentication, they sent back an invalid token.",
								pending_user.address(), pending_user.id()
							);
							Network::kick(&pending_user.address())?;
							return Ok(());
						}

						// The user has successfully authenticated!!
						log::info!(target: LOG, "Successfully authenticated {}", connection);

						// There /could/ be an edge case here where the timeout thread is processing that the player has timed out
						// after this process says they have authenticated but has not caused the thread to stop yet.
						// For now, this is fine because if its that close, the thread would still kick the client,
						// which would cause the `Disconnected` event and automatically cleanup the soon-to-be-active user in a later tick.

						// If there is not a race-condition, this will prevent the timeout from happening in the near future
						pending_user.stop_timeout();

						if let Ok(mut server) = self.server().write() {
							if server.find_user(&pending_user.id()).is_none() {
								let player_dir_path = server.get_players_dir_path();
								server.add_user(user::saved::User::new(
									&pending_user,
									player_dir_path,
								));
							}
						} else {
							return Err(Box::new(Error::CannotReadServerData));
						}

						let arc_world = self.entity_world.upgrade().unwrap();
						if let Ok(mut world) = arc_world.write() {
							log::debug!(
								"Initializing entity for new player({})",
								pending_user.id()
							);

							// Build an entity for the player which is marked with
							// the account id of the user and the ip address of the connection.
							let mut builder = archetype::player::Server::new()
								.with_user_id(pending_user.id().clone())
								.with_address(*pending_user.address())
								.build();

							// Integrated Client-Server needs to spawn client-only components
							// if its the local player's entity.
							if local_data.is_client() {
								let client_reg = account::ClientRegistry::read()?;
								let local_account = client_reg.active_account().unwrap();
								// If the account ids match, then this entity is the local player's avatar
								if *local_account.id() == *pending_user.id() {
									builder = archetype::player::Client::apply_to(builder);
								}
							}

							world.spawn(builder.build());
						}

						// Tell all clients (including self if CotoS) that a user has joined.
						Network::send_packets(
							Packet::builder()
								.with_mode(Broadcast)
								.with_guarantee(Reliable + Unordered)
								.with_payload(&Handshake(Request::ClientAuthenticated(
									pending_user.id().clone(),
								))),
						)?;

						if let Ok(mut active_cache) = self.active_cache.write() {
							let active_user = pending_user.into();
							active_cache.insert(active_user);
						}
					}
					Err(err) => {
						// if it doesnt match, the client isnt who they say they are,
						// so they should be kicked from the server (technically they've already connected).
						Network::kick(&pending_user.address())?;
						return Err(err);
					}
				}

				Ok(())
			}
			_ => Err(Box::new(Error::InvalidRequest(data.0.clone()))),
		}
	}
}

#[derive(Clone)]
struct ClientProcessor {
	app_state: crate::app::state::ArcLockMachine,
}

impl Processor for ClientProcessor {
	fn process(
		&self,
		kind: &event::Kind,
		data: &mut Option<event::Data>,
		local_data: &LocalData,
	) -> VoidResult {
		self.process_as(kind, data, local_data)
	}
}

impl PacketProcessor<Handshake> for ClientProcessor {
	#[profiling::function]
	fn process_packet(
		&self,
		_kind: &event::Kind,
		data: &mut Handshake,
		connection: &Connection,
		guarantee: &Guarantee,
		_local_data: &LocalData,
	) -> VoidResult {
		match &data.0 {
			Request::AuthTokenForClient(encrypted_bytes, server_public_key) => {
				profiling::scope!("received-auth-token");
				log::info!(target: LOG, "Received auth token from server");
				// Technically we will have "connected" by the end of this request,
				// but not really connected until the server validates the token.
				let reencrypted_bytes = if let Some(account::Account { key, .. }) =
					account::ClientRegistry::read()?.active_account()
				{
					let server_key = account::Key::from_string(&server_public_key)?;
					match key.decrypt(&encrypted_bytes)? {
						Ok(raw_token_bytes) => server_key.encrypt(&raw_token_bytes)?,
						Err(_) => {
							return Err(Box::new(Error::ClientTokenUnparsable));
						}
					}
				} else {
					return Err(Box::new(Error::NoActiveAccount));
				};

				data.0 = Request::AuthTokenForServer(reencrypted_bytes);
				Network::send_packets(
					Packet::builder()
						.with_address(connection.address)?
						.with_guarantee(*guarantee)
						.with_payload(data),
				)?;

				Ok(())
			}
			Request::ClientAuthenticated(account_id) => {
				let profiling_tag = format!("{}", account_id);
				profiling::scope!("client-authenticated", profiling_tag.as_str());
				let authenticated_self = account::ClientRegistry::read()?
					.active_account()
					.map(|account| account.meta.id == *account_id)
					.unwrap_or(false);
				log::debug!(
					target: LOG,
					"Client authenticated, authenticated_self:{} id:{}",
					authenticated_self,
					account_id
				);

				// TODO: If some other client has authed, add their account::Meta to some known-clients list for display in a "connected users" ui

				if authenticated_self {
					// TODO: Server should send a "all data is ready" signal to tell the client
					// that it is safe to enter the game, once relevant chunks and entities have been loaded.
					// Must require:
					// - player's entity and components have been replicated

					use crate::app::state::State::InGame;
					self.app_state.write().unwrap().transition_to(InGame, None);
				}

				Ok(())
			}
			_ => Err(Box::new(Error::InvalidRequest(data.0.clone()))),
		}
	}
}
