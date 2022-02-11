use socknet::connection::Connection;
use std::{net::SocketAddr, sync::Weak};

#[derive(Clone)]
pub enum Event {
	Created(SocketAddr, Weak<Connection>, /*is_local*/ bool),
	Dropped(SocketAddr),
}
