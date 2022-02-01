use engine::{
	network::socknet::{
		connection::{self, Connection},
		stream,
	},
	utility::Result,
};
use std::sync::Arc;

/// Builder context for entity replication stream
pub struct Builder {}

/// The stream handler id is `replication::entity`.
///
/// ```rust
/// use engine::network::socknet::stream::Identifier;
/// assert_eq!(Builder::unique::id(), "replication::entity");
/// ```
impl stream::Identifier for Builder {
	fn unique_id() -> &'static str {
		"replication::entity"
	}
}

/// Opening the handler results in an outgoing unidirectional stream
impl stream::send::Builder for Builder {
	type Opener = stream::uni::Opener;
}

/// Receiving the handler results in an incoming unidirectional stream
impl stream::recv::Builder for Builder {
	type Extractor = stream::uni::Extractor;
	type Receiver = super::recv::Handler;
}
