use std::net::SocketAddr;

#[derive(Clone, Copy, Debug)]
pub enum Event {
	Created(SocketAddr),
	Dropped(SocketAddr),
}
