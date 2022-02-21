//! Stream initiated by clients to join the server.
//! 
//! See [Identifier] for stream graph.

#[doc(hidden)]
mod identifier;
pub use identifier::*;

/// Context & Handler for the client/sender.
pub mod client;
/// Context & Handler for the server/receiver.
pub mod server;
