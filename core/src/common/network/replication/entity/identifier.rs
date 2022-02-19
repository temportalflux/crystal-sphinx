use socknet::stream;
use std::sync::Arc;

use crate::common::network::replication::entity::{client, server};

#[cfg_attr(doc, aquamarine::aquamarine)]
/// The identifier struct for the entity replication stream (`replication::entity`).
///
/// [Edit Diagram](https://mermaid.live/edit#pako:eNqdkk1OwzAQha8y8gqkcoEsKqG0EixgQcTOm8EeWqvJONiTQFT17tg4qEDbDSv_zPeen-3ZK-MtqUpFehuIDa0cbgJ2mrXgIJ6H7oVCXvUYxBnXIws0gBEaCuNpqc6lunXEkkuPXgh84qCp4IkMuZHsXIfbQbZpdAYTtB7z1lUkgjtkG7e4o-ts0XrfH6VDbzMtHqyLPYrZZubSQUc6EttCNjfLZV1B4zua62UfW0nClkYs0X-7ZkWP7ww5sEwFoDYSPP_w-KN4yPlmBeAGHUcBbAOhnSBmO7JnhcUT6MNFcbw5PfQ-hDkq-AArihL8dMHs3ymSbbny2RjlPb8GtVAdhQ6dTZ201wygVfrajrSq0tRi2Gml-ZC48uJr68QHVb1iusxC5U5rJjaqkjDQNzS34kwdPgFrzfu1)
/// ```mermaid
/// sequenceDiagram
/// 	autonumber
/// 	participant S as Server
/// 	participant C as Client
/// 	Note over S: Received Client Authenticate Event (see Handshake)
/// 	loop Received update to dispatch
/// 		Note over S: Received update to send
/// 		S->>C: Some update
/// 		alt Relevant
/// 			Note over C: Spawn entity
/// 		else Update
/// 			Note over C: Match entity against already spawned
/// 			Note over C: Update existing entity
/// 		else Irrelevant or Destroyed
/// 			Note over C: Match entity against already spawned
/// 			Note over C: Despawn existing entity
/// 		end
/// 	end
/// ```
pub struct Identifier {
	/// The (empty) application context for the server/sender.
	pub server: Arc<server::AppContext>,
	/// The application context for the client/receiver.
	pub client: Arc<client::AppContext>,
}

impl stream::Identifier for Identifier {
	type SendBuilder = server::AppContext;
	type RecvBuilder = client::AppContext;
	fn unique_id() -> &'static str {
		"replication::entity"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.server
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.client
	}
}
