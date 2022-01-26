use crate::common::network::CloseCode;
use engine::{
	network::socknet::{self, builder, connection::Connection, stream},
	utility::{self, Result},
};
use std::sync::{Arc, Weak};

#[builder(
	"handshake",
	Bidirectional,
	Handshake::process_client,
	Handshake::process_server
)]
pub struct Builder {}

pub struct Handshake {
	context: Arc<Builder>,
	connection: Arc<Connection>,
	send: stream::Send,
	recv: stream::Recv,
}

impl stream::Buildable for Handshake {
	type Builder = Builder;
	type Stream = (stream::Send, stream::Recv);
	fn build(
		context: Arc<Self::Builder>,
		connection: Arc<Connection>,
		(send, recv): Self::Stream,
	) -> Self {
		Self {
			context,
			connection,
			send,
			recv,
		}
	}
}

static PASSED_AUTH: u32 = 0;
static FAILED_AUTH: u32 = 1;

impl Handshake {
	pub fn builder() -> Builder {
		Builder {}
	}

	fn log(&self, side: &str) -> String {
		format!(
			"{}/{}[{}]",
			side,
			<Builder as stream::Builder>::unique_id(),
			self.connection.remote_address()
		)
	}

	async fn process_client(mut self) -> Result<()> {
		use utility::Context;
		let log = self.log("client");
		log::info!(target: &log, "Initiating handshake");

		// Tells the server how to process the stream (and establishes the stream).
		self.send
			.write_id::<Builder>()
			.await
			.context("writing handshake id")?;

		let key_pair = {
			use ring::signature::{self, EcdsaKeyPair};
			let source = self.connection.endpoint()?;
			let private_key = source.private_key();
			EcdsaKeyPair::from_pkcs8(&signature::ECDSA_P256_SHA256_ASN1_SIGNING, &private_key.0)
				.map_err(|err| KeyRejected(err.description_()))?
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

		// Step 3: Sign the random token & send it to the server.
		let token = self.recv.read_bytes().await.context("reading token")?;
		let signature = {
			use ring::rand::SystemRandom;

			let rng = SystemRandom::new();
			let signature = key_pair.sign(&rng, &token).map_err(|_| FailedToSignToken)?;

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

		Ok(())
	}

	async fn process_server(mut self) -> Result<()> {
		use utility::Context;
		let log = self.log("server");
		log::info!(target: &log, "Received handshake");

		// Step 1: Receive the client's public key
		// (which is derived from there private_key and is different from the certificate)
		let public_key = self.recv.read_bytes().await.context("reading public key")?;
		let encoded_key = socknet::utility::encode_string(&public_key);
		log::info!(target: &log, "Received public-key({})", encoded_key);

		// Step 2: Determine if the account has joined before
		// TODO: If the account (whose id is the certificate's fingerprint) has never joined before,
		// then they automatically pass the first phase.
		// Otherwise, the client-provided public key must match the public key stored to file.
		// To store to file: base64 encode the bytes of the client-provided public key.

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
			let public_key =
				UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &public_key);
			public_key.verify(&token, &signed_token).is_ok()
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

		// TODO: Process the user data/log them in

		Ok(())
	}
}

struct KeyRejected(&'static str);
impl std::error::Error for KeyRejected {}
impl std::fmt::Debug for KeyRejected {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for KeyRejected {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Key rejected during parsing: {}", self.0)
	}
}

struct FailedToSignToken;
impl std::error::Error for FailedToSignToken {}
impl std::fmt::Debug for FailedToSignToken {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for FailedToSignToken {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Failed to sign handshake token")
	}
}
