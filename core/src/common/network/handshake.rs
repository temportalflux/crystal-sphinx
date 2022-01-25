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
		static LOG: &'static str = "client/handshake";

		log::info!(
			target: LOG,
			"Initiating handshake to server({})",
			self.connection()?.fingerprint()?
		);

		// Tells the server how to process the stream (and establishes the stream).
		self.send.write_id::<Handshake>().await?;

		// Server has sent us a token that is encrypted both with its key and ours
		let token_bytes = self.recv.read::<Vec<u8>>().await?;

		let token_bytes = {
			log::debug!("decrypting with client key");
			let client_private_key = {
				use rsa::pkcs1::FromRsaPrivateKey;
				let source = self.connection()?.endpoint()?;
				let private_key = source.private_key();
				rsa::RsaPrivateKey::from_pkcs1_der(&private_key.0)?
			};

			{
				let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
				client_private_key.decrypt(padding, &token_bytes)?
			}
		};

		// Token is now partially decrypted.
		// If the server successfully decrypts,
		// then we become authenticated.
		self.send.write(&token_bytes).await?;

		let code = self.send.stopped().await?;
		assert_eq!(code, 0);

		Ok(())
	}

	async fn process_server(mut self) -> Result<()> {
		use rand::Rng;
		static LOG: &'static str = "server/handshake";

		log::info!(
			target: LOG,
			"Received handshake from client({})",
			self.connection()?.fingerprint()?
		);

		let raw_token: String = rand::thread_rng()
			.sample_iter(&rand::distributions::Alphanumeric)
			.take(64)
			.map(char::from)
			.collect();
		let token_bytes = bincode::serialize(&raw_token)?;
		
		let token_bytes = {
			log::debug!("encrypting with server key");
			let server_public_key = {
				use rsa::pkcs1::FromRsaPublicKey;
				let source = self.connection()?.endpoint()?;
				let certificate = source.certificate();
				rsa::RsaPublicKey::from_pkcs1_der(&certificate.0)?
			};

			{
				use rand::rngs::OsRng;
				use rsa::PublicKey;
				let mut rng = OsRng;
				let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
				server_public_key.encrypt(&mut rng, padding, &token_bytes)?
			}
		};

		let token_bytes = {
			log::debug!("encrypting with client key");
			let client_public_key = {
				use rsa::pkcs1::FromRsaPublicKey;
				let source = self.connection()?;
				let certificate = source.certificate()?;
				rsa::RsaPublicKey::from_pkcs1_der(&certificate.0)?
			};
	
			{
				use rand::rngs::OsRng;
				use rsa::PublicKey;
				let mut rng = OsRng;
				let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
				client_public_key.encrypt(&mut rng, padding, &token_bytes)?
			}
		};
	
		// Token is encrypted with server key and then client key.
		// Wait for client to send back the token decrypted from the client.
		self.send.write(&token_bytes).await?;
		
		// The client has sent pack a partially decrypted token.
		let token_bytes = self.recv.read::<Vec<u8>>().await?;

		let token_bytes = {
			log::debug!("decrypting with server key");
			let server_private_key = {
				use rsa::pkcs1::FromRsaPrivateKey;
				let source = self.connection()?.endpoint()?;
				let private_key = source.private_key();
				rsa::RsaPrivateKey::from_pkcs1_der(&private_key.0)?
			};

			{
				let padding = rsa::PaddingScheme::new_pkcs1v15_encrypt();
				server_private_key.decrypt(padding, &token_bytes)?
			}
		};

		let decrypted_token: String = bincode::deserialize(&token_bytes)?;

		let successful = decrypted_token == raw_token;
		log::debug!("handshake was successful? {}", successful);

		self.recv.stop(0).await?;
		self.send.finish().await?;

		Ok(())
	}
}

struct InvalidRsaKey;
impl std::error::Error for InvalidRsaKey {}
impl std::fmt::Debug for InvalidRsaKey {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for InvalidRsaKey {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Failed to parse encryption key")
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
