use crate::{
	engine::{
		network::{
			self,
			connection::Connection,
			event, mode,
			packet::{Guarantee, Packet},
			packet_kind,
			processor::{EventProcessors, PacketProcessor, Processor},
			LocalData, Network,
		},
		utility::VoidResult,
	}, account
};
use serde::{Deserialize, Serialize};

#[packet_kind(crate::engine::network)]
#[derive(Serialize, Deserialize)]
pub struct Handshake(Request);

#[derive(Serialize, Deserialize)]
enum Request {
	Login(account::Id, /*public key*/ String),
	AuthTokenForClient(/*encrypted auth token*/ String, /*server public key*/ String),
	AuthTokenForServer(/*re-encrypted auth token*/ String),
}

impl Handshake {
	pub fn register(builder: &mut network::Builder) {
		use mode::Kind::*;
		builder.register_bundle::<Handshake>(
			EventProcessors::default()
				.with(Server, ProcessAuthRequest())
				.with(mode::Set::all(), ProcessAuthRequest())
				.with(Client, ReEncryptAuthToken()),
		);
	}

	pub fn connect_to_server() -> VoidResult {
		use network::packet::{DeliveryGuarantee::*, OrderGuarantee::*};
		let request = match account::ClientRegistry::read()?.active_account() {
			Some(account) => Request::Login(
				account.id().clone(),
				account.public_key().as_string()?,
			),
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

struct ProcessAuthRequest();

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
			Request::Login(_account_id, public_key) => {
				// Requirements:
				// - server has a private (and therefore public) key
				// - server has a save game (so the saved-accounts can be checked/stored to)
				//   - this needs some understanding of how save games will be stored for both servers and clients
				// - need to create a pending-login manager
				// - need to add connect and disconnection protocols (maybe replace the existing ones so that connections are not formed until after auth?)

				// TODO: Check if the current save game has a user with the same id.
				// Add a pending request with the client's net address, account id, and public key.
				// If the id HAS logged in before, then compare the public keys. If they are not the same, don't respond (they will auto disconnect).
				// If they are the same (or there was no save data), then generate a random token encrypted with the client's public key.
				// By sending this packet back, we are allowing them to "connect" without auth,
				// so we can also clear the pending request if they disconnect.
				// Need to have some shared info for between the disconnect processor and this processor to contain this pending queue.
				// Also the server needs a public and private key.
				let token = "TODO: some random string".to_owned();
				let server_public_key = "TODO: server's public key".to_owned();
				data.0 = Request::AuthTokenForClient(token, server_public_key);
				Network::send(
					Packet::builder()
						.with_address(connection.address)?
						.with_guarantee(*guarantee)
						.with_payload(data)
						.build(),
				)?;
			}
			Request::AuthTokenForServer(reencrypted_token) => {
				// TODO: decrypt token, and if it matches, load the player into the server.
				// if it doesnt match, the client isnt who they say they are,
				// so they should be kicked from the server (technically they've already connected).
			}
			_ => {} // error
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
		if let Request::AuthTokenForClient(token, server_public_key) = &data.0 {
			// TODO: decrypt token with my private key, and re-encrypt with the server_public_key,
			// then send back to the server.
			// Technically we will have "connected" by the end of this,
			// but not really connected until the server validates the token.
			data.0 = Request::AuthTokenForServer(token.clone());
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
