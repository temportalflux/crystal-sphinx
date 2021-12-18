use super::{ArcLockCache, ArcLockChunk, Chunk, Level, LoadRequestReceiver, Ticket};
use engine::{
	math::nalgebra::Point3,
	utility::{spawn_thread, VoidResult},
};
use std::{
	collections::HashMap,
	sync::{Arc, Weak},
};

static LOG: &'static str = "chunk-loading";
pub type Handle = Arc<()>;

struct ThreadState {
	/// The public cache of chunks that are currently loaded.
	/// The cache holds no ownership of chunks,
	/// just weak references to what is loaded at any given time.
	cache: ArcLockCache,

	/// List of inactive and recently dropped tickets (and the chunk coordinates they reference).
	ticket_bindings: Vec<(Weak<Ticket>, Vec<Point3<i64>>)>,
	/// Map of coordinate to chunk states (and the actual strong reference to keep the chunk loaded).
	chunk_states: HashMap<Point3<i64>, ChunkTicketState>,

	chunk_expiration_duration: std::time::Duration,
	first_expiration_time: Option<std::time::Instant>,
	chunks_pending_unload: Vec<(std::time::Instant, Point3<i64>)>,

	no_incoming_request: bool,
	disconnected_from_requests: bool,
}

pub fn start(incoming_requests: LoadRequestReceiver, cache: &ArcLockCache) -> Handle {
	let handle = Handle::new(());

	let weak_handle = Arc::downgrade(&handle);

	let cache = cache.clone();
	spawn_thread(LOG, move || -> VoidResult {
		let mut thread_state = ThreadState {
			cache: cache.clone(),
			ticket_bindings: Vec::new(),
			chunk_states: HashMap::new(),
			chunk_expiration_duration: std::time::Duration::from_secs(60),
			first_expiration_time: None,
			chunks_pending_unload: Vec::new(),
			no_incoming_request: false,
			disconnected_from_requests: false,
		};

		// while the database/cache has not been discarded,
		// processing any pending load requests & unload any chunks no longer needed
		log::info!(target: LOG, "Starting chunk-loading thread");
		while weak_handle.strong_count() > 0 {
			thread_state.update(&incoming_requests);
			std::thread::sleep(std::time::Duration::from_millis(1));
		}
		log::info!(target: LOG, "Ending chunk-loading thread");

		Ok(())
	});

	handle
}

impl ThreadState {
	fn update(&mut self, incoming_requests: &LoadRequestReceiver) {
		self.process_new_tickets(&incoming_requests);
		self.update_dropped_tickets();
		if self.has_expired_chunks() {
			let chunks_for_unloading = self.find_expired_chunks();
			self.unload_expired_chunks(chunks_for_unloading);
		}
	}

	fn process_new_tickets(&mut self, incoming_requests: &LoadRequestReceiver) {
		use crossbeam_channel::TryRecvError;
		self.no_incoming_request = false;
		while !self.disconnected_from_requests && !self.no_incoming_request {
			match incoming_requests.try_recv() {
				Ok(weak_ticket) => {
					self.sync_process_ticket(weak_ticket);
				}
				// no events, continue the loop after a short nap
				Err(TryRecvError::Empty) => {
					self.no_incoming_request = true;
				}
				// If disconnected, then kill the thread
				Err(TryRecvError::Disconnected) => {
					log::debug!(target: "chunk-loading", "Disconnected from chunk request channel");
					self.disconnected_from_requests = true;
				}
			}
		}
	}

	fn sync_process_ticket(&mut self, weak_ticket: Weak<Ticket>) {
		let arc_ticket = match weak_ticket.upgrade() {
			Some(ticket) => ticket,
			None => return, // early out if the user has already dropped the ticket
		};
		let processed_chunks = self.sync_load_ticket_chunks(arc_ticket);
		let mut ticket_chunks = Vec::with_capacity(processed_chunks.len());
		for (coordinate, arc_chunk, level) in processed_chunks.into_iter() {
			self.insert_or_update_chunk_state(&weak_ticket, coordinate, level, &arc_chunk);
			ticket_chunks.push(coordinate);
		}
		self.ticket_bindings.push((weak_ticket, ticket_chunks));
	}

	fn sync_load_ticket_chunks(
		&mut self,
		ticket: Arc<Ticket>,
	) -> Vec<(Point3<i64>, ArcLockChunk, Level)> {
		let mut chunks = Vec::new();
		let coordinate_levels = ticket.coordinate_levels();
		for (coordinate, level) in coordinate_levels.into_iter() {
			let arc_chunk = self.sync_load_chunk(coordinate, level);
			chunks.push((coordinate, arc_chunk, level));
		}
		chunks
	}

	fn sync_load_chunk(&mut self, coordinate: Point3<i64>, level: Level) -> ArcLockChunk {
		let loaded_chunk = self
			.cache
			.read()
			.unwrap()
			.find(&coordinate)
			.map(|arc| arc.clone());
		let (_freshly_loaded, arc_chunk) = match loaded_chunk {
			Some(weak_chunk) => {
				let some_arc_chunk = weak_chunk.upgrade();
				assert!(some_arc_chunk.is_some());
				(false, some_arc_chunk.unwrap())
			}
			None => {
				let mut cache = self.cache.write().unwrap();
				let arc_chunk =
					Chunk::load_or_generate(&coordinate, level, &cache.world_gen_settings);
				cache.insert(&coordinate, Arc::downgrade(&arc_chunk));
				(true, arc_chunk)
			}
		};

		arc_chunk
	}

	fn insert_or_update_chunk_state(
		&mut self,
		weak_ticket: &Weak<Ticket>,
		coordinate: Point3<i64>,
		level: Level,
		arc_chunk: &ArcLockChunk,
	) {
		match self.chunk_states.get_mut(&coordinate) {
			Some(state) => {
				if state.level < level {
					state.level = level;
				}
				state.tickets.push(weak_ticket.clone());
			}
			None => {
				self.chunk_states.insert(
					coordinate,
					ChunkTicketState {
						chunk: arc_chunk.clone(),
						level: level,
						tickets: vec![weak_ticket.clone()],
					},
				);
			}
		}
	}

	/// Iterate over all bound tickets to detect if any have been dropped.
	/// All chunks related to a dropped ticket, which aren't referenced by other tickets,
	/// are sent to the pending_unload list.
	fn update_dropped_tickets(&mut self) {
		let now = std::time::Instant::now();
		// Can use `Vec::drain_filter` when that api stabilizes.
		// O(n) performance where `n` is the number of loaded chunks
		let mut i = 0;
		while i < self.ticket_bindings.len() {
			// if there are no strong references, the ticket has been dropped
			if self.ticket_bindings[i].0.strong_count() == 0 {
				// dropped tickets mean their chunks should be moved to a pending list of chunks that will be removed soon
				let (_dropped_ticket, chunks) = self.ticket_bindings.remove(i);
				// Iterate over all the chunks that the dropped ticket referenced
				for coordinate in chunks {
					let state = self.chunk_states.get_mut(&coordinate).unwrap();
					// If the chunk state indicates that no other tickets requested this chunk,
					// then move the chunk to the list of chunks to be unloaded in the near future.
					// This list is always sorted by timestamp such that the earliest are first.
					if state.update() {
						if self.first_expiration_time.is_none() {
							self.first_expiration_time = Some(now);
						}
						self.chunks_pending_unload.push((now, coordinate));
					}
				}
			} else {
				i += 1;
			}
		}
	}

	fn has_expired_chunks(&self) -> bool {
		match self.first_expiration_time {
			Some(insertion_time) => {
				insertion_time.duration_since(std::time::Instant::now())
					> self.chunk_expiration_duration
			}
			None => false,
		}
	}

	fn find_expired_chunks(&mut self) -> Vec<ArcLockChunk> {
		// Invalidate the flag indicating that there is some chunk pending expiration.
		// We will later give it a proper value if there are still chunks in the list which will expire later.
		self.first_expiration_time = None;

		let mut chunks_for_unloading = Vec::new();
		let now = std::time::Instant::now();
		// Can use `Vec::drain_filter` when that api stabilizes.
		// O(n) performance where `n` is the number of loaded chunks
		let mut i = 0;
		while i < self.chunks_pending_unload.len() {
			let (insertion_time, coordinate) = self.chunks_pending_unload[i].clone();
			// A chunk has expired if the amount of time since insertion exceeds the maximum.
			let has_expired = insertion_time.duration_since(now) > self.chunk_expiration_duration;
			// If the chunk has any new tickets which reference it, it shouldnt be dropped.
			let has_been_renewed = if let Some(state) = self.chunk_states.get(&coordinate) {
				state.tickets.len() > 0
			} else {
				unimplemented!()
			};
			// If any given chunk /should be dropped/ or a new ticket has referenced it, it is no longer "pending" unload.
			if has_expired || has_been_renewed {
				self.chunks_pending_unload.remove(i);
				// Only move the chunk to the list to be unloaded if no other tickets reference it.
				if !has_been_renewed {
					let state = self.chunk_states.remove(&coordinate).unwrap();
					chunks_for_unloading.push(state.chunk);
				}
			} else {
				i += 1;
				// There is still an element in the list, so make sure the next time we iterate
				// is the earliest moment of the next expiration (but no earlier).
				if self.first_expiration_time.is_none()
					|| insertion_time < self.first_expiration_time.unwrap()
				{
					self.first_expiration_time = Some(insertion_time);
				}
			}
		}
		chunks_for_unloading
	}

	fn unload_expired_chunks(&mut self, mut chunks_for_unloading: Vec<ArcLockChunk>) {
		// Unload each chunk, dropping them one-after-one after each iteration
		if !chunks_for_unloading.is_empty() {
			log::debug!(
				target: LOG,
				"Unloading {} chunks",
				chunks_for_unloading.len()
			);
			for arc_chunk in chunks_for_unloading.drain(..) {
				let chunk = arc_chunk.read().unwrap();
				self.cache.write().unwrap().remove(chunk.coordinate());
				chunk.save()
			}
		}
	}
}

pub(crate) struct ChunkTicketState {
	pub(crate) chunk: ArcLockChunk,
	pub(crate) level: Level,
	pub(crate) tickets: Vec<Weak<Ticket>>,
}

impl ChunkTicketState {
	pub fn update(&mut self) -> bool {
		let mut i = 0;
		let mut highest_level = None;
		while i < self.tickets.len() {
			match self.tickets[i].upgrade() {
				None => {
					self.tickets.remove(i);
				}
				Some(arc_ticket) => {
					i += 1;
					let ticket_level: Level = arc_ticket.level.into();
					if highest_level.is_none() || ticket_level > highest_level.unwrap() {
						highest_level = Some(ticket_level);
					}
				}
			}
		}
		match highest_level {
			Some(level) => {
				if self.level != level {
					self.level = level;
					self.chunk.write().unwrap().level = level;
				}
				false
			}
			None => true,
		}
	}
}
