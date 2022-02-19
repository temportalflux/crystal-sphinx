use std::sync::Arc;

use socknet::stream;

use crate::common::network::replication::world::relevancy::{client, server};

#[cfg_attr(doc, aquamarine::aquamarine)]
/// The identifier struct for the world-relevancy stream (`replication::chunk-relv`).
///
/// [Edit Diagram](https://mermaid.live/edit#pako:eNptksFqwzAMhl9F-LRB9wI5FEbW6xgLu_ni2kpi6kiu7bSU0nefsiRbYb3Z6Pv9_5J1VZYdqkplPI5IFt-86ZIZNOlixsI0DntM0y2aVLz10VCBBkyGBtPpf6meSnXwSGUqBeYIdW-oQygMCQOeJsz2Ix0yOCxoC7oJ1aV52W7rCr6iMwXhc2YtwlPwuQC3kGOPCfPzjL-zUCwZQEQfmFpOw-pgL-B82z4Al9cDWxN-afHIDITnNZhJCMZajBJusaslXVPdxXq1B-JzQNetDfwZCbgjmeiI64vS_R7FMAZvJYCD_QUic3gQseEk7Qa3SoXMviPfilJm52QaU4AHytXzTjz7isaa5OagSE5t1IBpMN7J3181AWhVehxQq0qOzqSDVppuwo0_A9s5XzipqjUh40ZNu9FcyKqqpBFXaFmehbp9A5NR20U)
/// ```mermaid
/// sequenceDiagram
/// 	autonumber
/// 	participant S as Server
/// 	participant C as Client
/// 	loop server::Sender::send_until_closed
/// 		S->>C: Update Relevance (list of spheres)
/// 		Note over C: Perform relevancy diff
/// 		Note over C: Update local relevance (so new chunks are accepted)
/// 		C->>S: Relevance Acknowledged
/// 		Note over S: Enqueue chunks to be replicated by pool
/// 		Note over C: Sort old chunks by significant distance
/// 		Note over C: Enqueue old chunks to be discarded
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
		"replication::chunk-relv"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.server
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.client
	}
}
