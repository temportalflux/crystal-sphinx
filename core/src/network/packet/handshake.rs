use crate::{account, server};
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

use crate::server::user;

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
			Self::ClientAuthenticated(account_id) => write!(f, "Authenticated(id={})", account_id),
		}
	}
}

#[derive(Debug, Clone)]
enum Error {
	InvalidRequest(Request),
	NoServerData,
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
			Self::NoServerData => write!(f, "Cannot process handshake, no server data."),
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
	) {
		use mode::Kind::*;
		builder.register_bundle::<Handshake>(
			EventProcessors::default()
				.with(
					Server,
					ProcessAuthRequest {
						auth_cache: auth_cache.clone(),
						active_cache: active_cache.clone(),
					},
				)
				.with(Client, ReEncryptAuthToken())
				.with(
					mode::Set::all(),
					AnyProcessor::new(vec![
						ProcessAuthRequest {
							auth_cache: auth_cache.clone(),
							active_cache: active_cache.clone(),
						}
						.boxed(),
						ReEncryptAuthToken().boxed(),
					]),
				),
		);
	}

	pub fn connect_to_server() -> VoidResult {
		use network::prelude::*;
		let request = match account::ClientRegistry::read()?.active_account() {
			Some(account) => {
				Request::Login(account.meta.clone(), account.public_key().as_string()?)
			}
			None => return Ok(()),
		};
		Network::send_packets(
			Packet::builder()
				.with_address("127.0.0.1:25565")?
				.with_guarantee(Reliable + Unordered)
				.with_payload(&Handshake(request)),
		)
	}
}

struct ProcessAuthRequest {
	auth_cache: user::pending::ArcLockCache,
	active_cache: user::active::ArcLockCache,
}

impl Processor for ProcessAuthRequest {
	fn process(
		&self,
		kind: &event::Kind,
		data: &mut Option<event::Data>,
		local_data: &LocalData,
	) -> VoidResult {
		self.process_as(kind, data, local_data)
	}
}

impl PacketProcessor<Handshake> for ProcessAuthRequest {
	fn process_packet(
		&self,
		_kind: &event::Kind,
		data: &mut Handshake,
		connection: &Connection,
		guarantee: &Guarantee,
		_local_data: &LocalData,
	) -> VoidResult {
		match &data.0 {
			Request::Login(account_meta, public_key) => {
				log::info!(
					target: LOG,
					"Received login request from {}({})",
					connection.address,
					account_meta.id
				);
				let (server_auth_key, user) = match server::Server::read() {
					Ok(guard) => match &*guard {
						Some(server) => (
							server.auth_key().clone(),
							server.find_user(&account_meta.id).cloned(),
						),
						None => {
							return Err(Box::new(Error::NoServerData));
						}
					},
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
				log::info!(
					target: LOG,
					"Received auth token from {}",
					connection.address
				);

				// Wrapper function to try to decrypt an auth token,
				// so that the error can be handled gracefully
				// without sacrificing readability.
				fn decrypt_token(bytes: &[u8]) -> Result<String, AnyError> {
					let server_auth_key = match server::Server::read() {
						Ok(guard) => match &*guard {
							Some(server) => server.auth_key().clone(),
							None => {
								return Err(Box::new(Error::NoServerData));
							}
						},
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
				match decrypt_token(&reencrypted_token) {
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

						if let Ok(mut guard) = server::Server::write() {
							if let Some(server) = &mut *guard {
								if server.find_user(&pending_user.id()).is_none() {
									server.add_user(user::saved::User::new(
										&pending_user,
										server.get_players_dir_path(),
									));
								}
							} else {
								return Err(Box::new(Error::NoServerData));
							}
						} else {
							return Err(Box::new(Error::CannotReadServerData));
						}

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

struct ReEncryptAuthToken();

impl Processor for ReEncryptAuthToken {
	fn process(
		&self,
		kind: &event::Kind,
		data: &mut Option<event::Data>,
		local_data: &LocalData,
	) -> VoidResult {
		self.process_as(kind, data, local_data)
	}
}

impl PacketProcessor<Handshake> for ReEncryptAuthToken {
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
				Ok(())
			}
			_ => Err(Box::new(Error::InvalidRequest(data.0.clone()))),
		}
	}
}
