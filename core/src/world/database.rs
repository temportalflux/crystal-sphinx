use super::{
	chunk::{
		self,
		cache::server::{ArcLockServerCache, ServerCache},
	},
	Settings,
};
use engine::{
	math::nalgebra::Point3,
	utility::{AnyError, VoidResult},
};
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
	chunk_cache: ArcLockServerCache,
	_load_request_sender: Arc<chunk::ticket::Sender>,
	// When this is dropped, the loading thread stops.
	_chunk_thread_handle: chunk::thread::Handle,

	held_tickets: Vec<Arc<chunk::Ticket>>,
}

impl Database {
	pub fn new(root_path: PathBuf) -> Self {
		let settings = Settings::load(&root_path).unwrap();

		let chunk_cache = Arc::new(RwLock::new(ServerCache::new()));

		let (load_request_sender, load_request_receiver) = crossbeam_channel::unbounded();
		let thread_handle = chunk::thread::start(root_path, load_request_receiver, &chunk_cache);

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

	fn ticket_sender_static() -> &'static mut Option<Weak<chunk::ticket::Sender>> {
		static mut TICKET_SENDER: Option<Weak<chunk::ticket::Sender>> = None;
		unsafe { &mut TICKET_SENDER }
	}

	fn ticket_sender() -> Result<Arc<chunk::ticket::Sender>, AnyError> {
		Ok(Self::ticket_sender_static()
			.as_ref()
			.map(|weak| weak.upgrade())
			.flatten()
			.ok_or(NoWorldDatabase)?)
	}

	pub(crate) fn send_chunk_ticket(ticket: &Arc<chunk::Ticket>) -> VoidResult {
		Ok(Self::ticket_sender()?.try_send(Arc::downgrade(&ticket))?)
	}

	pub fn chunk_cache(&self) -> &ArcLockServerCache {
		&self.chunk_cache
	}

	pub fn load_origin_chunk(arc_world: &ArcLockDatabase) -> VoidResult {
		arc_world.write().unwrap().held_tickets.push(
			chunk::Ticket {
				coordinate: Point3::new(0, 0, 0),
				level: (chunk::Level::Ticking, 2).into(),
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
