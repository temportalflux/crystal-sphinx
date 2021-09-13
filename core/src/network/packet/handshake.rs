use crate::{account, server};
use engine::{
	network::{
		self,
		connection::Connection,
		event, mode,
		packet::{Guarantee, Packet},
		packet_kind,
		processor::{EventProcessors, PacketProcessor, Processor},
		LocalData, Network, LOG,
	},
	utility::VoidResult,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::server::user::pending::ArcLockAuthCache;

#[packet_kind(engine::network)]
#[derive(Serialize, Deserialize)]
pub struct Handshake(Request);

#[derive(Serialize, Deserialize)]
enum Request {
	Login(account::Id, /*public key*/ String),
	AuthTokenForClient(
		/*encrypted auth token*/ Vec<u8>,
		/*server public key*/ String,
	),
	AuthTokenForServer(/*re-encrypted auth token*/ Vec<u8>),
}

impl Handshake {
	pub fn register(builder: &mut network::Builder, auth_cache: &ArcLockAuthCache) {
		use mode::Kind::*;
		builder.register_bundle::<Handshake>(
			EventProcessors::default()
				.with(
					Server,
					ProcessAuthRequest {
						auth_cache: auth_cache.clone(),
					},
				)
				.with(
					mode::Set::all(),
					ProcessAuthRequest {
						auth_cache: auth_cache.clone(),
					},
				)
				.with(Client, ReEncryptAuthToken()),
		);
	}

	pub fn connect_to_server() -> VoidResult {
		use network::packet::{DeliveryGuarantee::*, OrderGuarantee::*};
		let request = match account::ClientRegistry::read()?.active_account() {
			Some(account) => {
				Request::Login(account.id().clone(), account.public_key().as_string()?)
			}
			None => return Ok(()),
		};
		Network::send(
			Packet::builder()
				.with_address("127.0.0.1:25565")?
				.with_guarantee(Reliable + Unordered)
				.with_payload(&Handshake(request))
				.build(),
		)
	}
}

struct ProcessAuthRequest {
	auth_cache: ArcLockAuthCache,
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
		local_data: &LocalData,
	) -> VoidResult {
		match &data.0 {
			Request::Login(account_id, public_key) => {
				let (server_auth_key, user) = match server::Server::read() {
					Ok(guard) => match &*guard {
						Some(server) => (
							server.auth_key().clone(),
							server.find_user(&account_id).cloned(),
						),
						None => {
							log::error!(
								target: network::LOG,
								"Cannot process handshake login, not a server."
							);
							return Ok(());
						}
					},
					Err(_) => {
						log::error!(target: network::LOG, "Cannot read from server data.");
						return Ok(());
					}
				};

				// Auto-deny the logic in the public key stored locally doesnt match the provided one.
				if let Some(arclock_user) = user {
					if let Ok(saved_user_guard) = arclock_user.read() {
						if *public_key != saved_user_guard.public_key().as_string()? {
							// The server intentionally does not respond, which will cause the client to timeout.
							log::info!(target: LOG, "Client {} tried to authenticate, but their public key did not match a previous login.", account_id);
							return Ok(());
						}
					}
				}

				let token: String = rand::thread_rng()
					.sample_iter(&rand::distributions::Alphanumeric)
					.take(64)
					.map(char::from)
					.collect();

				let server_public_key = server_auth_key.public();
				let encrypted_bytes = if let account::Key::Public(rsa_public) = &server_public_key {
					use rand::rngs::OsRng;
					use rsa::PublicKey;
					let mut rng = OsRng;
					let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
					rsa_public.encrypt(&mut rng, padding, token.as_bytes())?
				} else {
					// This should never happen, because we requested the public key from the auth key.
					log::error!(target: LOG, "FATAL: Server's auth key could not become a public key... something blew up... X_X");
					return Ok(());
				};

				let pending_user = server::user::pending::User::new(
					connection.address,
					account_id.clone(),
					account::Key::from_string(&public_key)?,
					token,
				);

				// NOTE: By sending this to the client, they will become connected (thats just how the network system works).
				// So if they fail auth, we need to kick them from the server.
				// If they disconnect (on their end or from getting kicked), we need to clear the pending request.
				data.0 =
					Request::AuthTokenForClient(encrypted_bytes, server_public_key.as_string()?);
				Network::send(
					Packet::builder()
						.with_address(connection.address)?
						.with_guarantee(*guarantee)
						.with_payload(data)
						.build(),
				)?;

				// It is impossible to receive a response from the client since sending the packet
				// (execution of this process is blocking further packets being processsed),
				// so its safe to add to the cache after sending the packet.
				// We need to execute after packet send so that if there are any errors in sending the packet,
				// we dont have a lingering pending entry.
				if let Ok(mut auth_cache) = self.auth_cache.write() {
					auth_cache.add_pending_user(pending_user);
				}
			}
			Request::AuthTokenForServer(_reencrypted_token) => {
				// TODO: decrypt token, and if it matches, load the player into the server.
				// if it doesnt match, the client isnt who they say they are,
				// so they should be kicked from the server (technically they've already connected).

				// TODO: If this request isn't received for x time after sending the AuthTokenForClient,
				// then the client needs to be kicked.
			}
			_ => {} // TODO: error invalid request
		}
		Ok(())
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
		local_data: &LocalData,
	) -> VoidResult {
		if let Request::AuthTokenForClient(encrypted_bytes, _server_public_key) = &data.0 {
			// TODO: decrypt token with my private key, and re-encrypt with the server_public_key,
			// then send back to the server.
			// Technically we will have "connected" by the end of this,
			// but not really connected until the server validates the token.
			data.0 = Request::AuthTokenForServer(encrypted_bytes.clone());
			Network::send(
				Packet::builder()
					.with_address(connection.address)?
					.with_guarantee(*guarantee)
					.with_payload(data)
					.build(),
			)?;
		}

		Ok(())
	}
}
