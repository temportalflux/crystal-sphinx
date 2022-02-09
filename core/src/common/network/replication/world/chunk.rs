use engine::network::socknet::stream;

pub mod client;
pub mod server;

pub struct Builder {}

impl stream::Identifier for Builder {
	fn unique_id() -> &'static str {
		"replication::world::chunk"
	}
}

impl stream::send::Builder for Builder {
	type Opener = stream::uni::Opener;
}

impl stream::recv::Builder for Builder {
	type Extractor = stream::uni::Extractor;
	type Receiver = client::Chunk;
}
