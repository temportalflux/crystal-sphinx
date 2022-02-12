use crate::common::network::connection;
use socknet::{self, connection::Connection, stream, utility::PinFutureResult};
use std::{
	collections::HashSet,
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

pub struct Broadcast<T> {
	connection_list: Arc<RwLock<connection::List>>,
	ignored_addresses: HashSet<SocketAddr>,
	on_established: Option<Arc<dyn Fn(T) -> PinFutureResult<()> + 'static + Send + Sync>>,
	_marker: std::marker::PhantomData<T>,
}

impl<T> Broadcast<T>
where

		T: Sized + stream::handler::Initiator
		+ From<stream::send::Context<<T::Identifier as stream::Identifier>::SendBuilder>>
		+ 'static
		+ Sized
		+ Send,
	T::Identifier: stream::Identifier + Send + Sync + 'static,
	<T::Identifier as stream::Identifier>::SendBuilder:
		stream::send::AppContext + Send + Sync + 'static,
	<<T::Identifier as stream::Identifier>::SendBuilder as stream::send::AppContext>::Opener:
		stream::Opener,
		<<<T::Identifier as stream::Identifier>::SendBuilder as stream::send::AppContext>::Opener as stream::Opener>::Output: stream::kind::send::Write + Send,

{
	pub fn new(connection_list: Arc<RwLock<connection::List>>) -> Self {
		Self {
			connection_list,
			ignored_addresses: HashSet::new(),
			on_established: None,
			_marker: std::marker::PhantomData::<T>::default(),
		}
	}

	pub fn ignore(mut self, connection: Arc<Connection>) -> Self {
		use socknet::connection::Active;
		self.ignored_addresses.insert(connection.remote_address());
		self
	}

	pub fn with_on_established<F>(mut self, when_established: F) -> Self
	where
		F: Fn(T) -> PinFutureResult<()> + 'static + Send + Sync,
	{
		let when_established = Arc::new(when_established);
		self.on_established = Some(when_established);
		self
	}

	pub fn make_target_list(&self) -> Vec<Weak<Connection>> {
		let connection_list = self.connection_list.read().unwrap();
		connection_list
			.all()
			.iter()
			.filter_map(
				|(address, connection)| match self.ignored_addresses.contains(&address) {
					true => None,
					false => Some(connection.clone()),
				},
			)
			.collect()
	}

	pub fn open(self) {
		assert!(self.on_established.is_some());
		for connection in self.make_target_list() {
			let arc = match Connection::upgrade(&connection) {
				Ok(arc) => arc,
				Err(_) => continue,
			};
			let when_established = self.on_established.as_ref().unwrap().clone();
			arc.spawn(async move {
				let handler = T::open(&connection)?.await?;
				(when_established)(handler).await?;
				Ok(())
			});
		}
	}
}
