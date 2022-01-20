use crate::server::world::{
	chunk::{cache, thread, ticket, Level, Ticket},
	Settings,
};
use engine::{math::nalgebra::Point3, utility::Result};
use std::{
	path::PathBuf,
	sync::{Arc, RwLock, Weak},
};

/// Alias for Arc<RwLock<[`Database`](Database)>>.
pub type ArcLockDatabase = Arc<RwLock<Database>>;

/// The data about a world (its chunks, settings, etc).
/// Exists on the server, does not contain presentational/graphical data.
pub struct Database {
	_settings: Settings,
	chunk_cache: cache::ArcLock,
	_load_request_sender: Arc<ticket::Sender>,
	// When this is dropped, the loading thread stops.
	_chunk_thread_handle: thread::Handle,

	held_tickets: Vec<Arc<Ticket>>,
}

impl Database {
	pub fn new(root_path: PathBuf) -> Self {
		let settings = Settings::load(&root_path).unwrap();

		let chunk_cache = Arc::new(RwLock::new(cache::Cache::new()));

		let (load_request_sender, load_request_receiver) = crossbeam_channel::unbounded();
		let thread_handle = thread::start(root_path, load_request_receiver, &chunk_cache);

		let load_request_sender = Arc::new(load_request_sender);
		*Self::ticket_sender_static() = Some(Arc::downgrade(&load_request_sender));

		Self {
			_settings: settings,
			chunk_cache,
			_load_request_sender: load_request_sender,
			_chunk_thread_handle: thread_handle,

			held_tickets: Vec::new(),
		}
	}

	fn ticket_sender_static() -> &'static mut Option<Weak<ticket::Sender>> {
		static mut TICKET_SENDER: Option<Weak<ticket::Sender>> = None;
		unsafe { &mut TICKET_SENDER }
	}

	fn ticket_sender() -> Result<Arc<ticket::Sender>> {
		Ok(Self::ticket_sender_static()
			.as_ref()
			.map(|weak| weak.upgrade())
			.flatten()
			.ok_or(NoWorldDatabase)?)
	}

	pub(crate) fn send_chunk_ticket(ticket: &Arc<Ticket>) -> Result<()> {
		Ok(Self::ticket_sender()?.try_send(Arc::downgrade(&ticket))?)
	}

	pub fn chunk_cache(&self) -> &cache::ArcLock {
		&self.chunk_cache
	}

	pub fn load_origin_chunk(arc_world: &ArcLockDatabase) -> Result<()> {
		arc_world.write().unwrap().held_tickets.push(
			Ticket {
				coordinate: Point3::new(0, 0, 0),
				level: (Level::Ticking, 2).into(),
			}
			.submit()?,
		);
		Ok(())
	}
}

impl Drop for Database {
	fn drop(&mut self) {
		*Self::ticket_sender_static() = None;
	}
}

#[derive(Debug)]
struct NoWorldDatabase;
impl std::error::Error for NoWorldDatabase {}
impl std::fmt::Display for NoWorldDatabase {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "No world database")
	}
}
