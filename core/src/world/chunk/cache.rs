use super::{ArcLockChunk, Chunk, WeakLockChunk};
use crate::world::Settings;
use crossbeam_channel::{Receiver, Sender};
use engine::{
	math::nalgebra::Point3,
	utility::{spawn_thread, VoidResult},
};
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, Mutex, RwLock},
};

pub type ArcLockCache = Arc<RwLock<Cache>>;

#[derive(Clone)]
pub struct GeneratorSettings {
	pub(super) root_dir: PathBuf,
	pub(super) _seed: String,
}

pub struct Cache {
	loaded_chunks: HashMap<Point3<i64>, WeakLockChunk>,
	world_gen_settings: GeneratorSettings,
}
impl Cache {
	pub(crate) fn new(settings: &Settings) -> Self {
		Self {
			loaded_chunks: HashMap::new(),
			world_gen_settings: GeneratorSettings {
				root_dir: settings.root_path().to_owned(),
				_seed: settings.seed().to_owned(),
			},
		}
	}

	pub fn find(&self, coordinate: &Point3<i64>) -> Option<&WeakLockChunk> {
		self.loaded_chunks.get(coordinate)
	}

	fn insert(&mut self, coordinate: &Point3<i64>, chunk: WeakLockChunk) {
		let old_value = self.loaded_chunks.insert(*coordinate, chunk);
		assert!(old_value.is_none());
	}

	fn remove(&mut self, coordinate: &Point3<i64>) {
		let old_value = self.loaded_chunks.remove(coordinate);
		assert!(old_value.is_some());
	}
}

pub type ThreadHandle = Arc<()>;
pub type LoadRequestSender = Sender<ArctexTicket>;
pub type LoadRequestReceiver = Receiver<ArctexTicket>;
pub type ArctexTicket = Arc<Mutex<Ticket>>;
pub struct Ticket {
	coordinate: Point3<i64>,
	chunk_handle: Option<ArcLockChunk>,
}
impl Ticket {
	pub fn new(coordinate: Point3<i64>) -> Self {
		Self {
			coordinate,
			chunk_handle: None,
		}
	}
	pub fn coordinate(&self) -> &Point3<i64> {
		&self.coordinate
	}
	fn set_chunk(&mut self, handle: ArcLockChunk) {
		self.chunk_handle = Some(handle);
	}
}

impl Cache {
	pub fn start_loading_thread(
		incoming_requests: LoadRequestReceiver,
		cache: &ArcLockCache,
	) -> ThreadHandle {
		let handle = ThreadHandle::new(());

		let weak_handle = Arc::downgrade(&handle);
		let thread_cache = cache.clone();
		spawn_thread("chunk-loading", move || -> VoidResult {
			use crossbeam_channel::TryRecvError;

			log::info!(target: "chunk-loading", "Starting chunk-loading thread");

			let mut loaded_chunks = vec![];
			let mut chunks_for_unloading = vec![];
			let mut no_incoming_request;
			let mut disconnected_from_requests = false;

			// while the database/cache has not been discarded,
			// processing any pending load requests & unload any chunks no longer needed
			while weak_handle.strong_count() > 0 {
				// Process all incoming load requests/tickets
				no_incoming_request = false;
				while !disconnected_from_requests && !no_incoming_request {
					match incoming_requests.try_recv() {
						Ok(ticket) => {
							match Cache::sync_load_chunk(&thread_cache, ticket) {
								(/*freshly_loaded*/ true, arc_chunk) => {
									loaded_chunks.push(arc_chunk);
								}
								_ => {}
							}
						}
						// no events, continue the loop after a short nap
						Err(TryRecvError::Empty) => {
							no_incoming_request = true;
						}
						// If disconnected, then kill the thread
						Err(TryRecvError::Disconnected) => {
							log::debug!(target: "chunk-loading", "Disconnected from chunk request channel");
							disconnected_from_requests = true;
						}
					}
				}

				// Can use `Vec::drain_filter` when that api stabilizes.
				// O(n) performance where `n` is the number of loaded chunks
				let mut i = 0;
				while i < loaded_chunks.len() {
					// when the only strong reference is the one in the loaded_chunks array, then all tickets have been discarded
					if Arc::strong_count(&loaded_chunks[i]) == 1 {
						chunks_for_unloading.push(loaded_chunks.remove(i));
					} else {
						i += 1;
					}
				}

				// Unload each chunk, dropping them one-after-one after each iteration
				if !chunks_for_unloading.is_empty() {
					log::debug!("Unloading {} chunks", chunks_for_unloading.len());
					for arc_chunk in chunks_for_unloading.drain(..) {
						let chunk = arc_chunk.read().unwrap();
						thread_cache.write().unwrap().remove(chunk.coordinate());
						chunk.save()
					}
				}

				std::thread::sleep(std::time::Duration::from_millis(1));
			}
			log::info!(target: "chunk-loading", "Ending chunk-loading thread");
			Ok(())
		});

		handle
	}

	fn sync_load_chunk(cache: &ArcLockCache, ticket: ArctexTicket) -> (bool, ArcLockChunk) {
		let coordinate: Point3<i64> = *ticket.lock().unwrap().coordinate();

		let loaded_chunk = cache
			.read()
			.unwrap()
			.find(&coordinate)
			.map(|arc| arc.clone());
		let (freshly_loaded, arc_chunk) = match loaded_chunk {
			Some(weak_chunk) => {
				let some_arc_chunk = weak_chunk.upgrade();
				assert!(some_arc_chunk.is_some());
				(false, some_arc_chunk.unwrap())
			}
			None => {
				let mut cache = cache.write().unwrap();
				let arc_chunk = Chunk::load_or_generate(&coordinate, &cache.world_gen_settings);
				cache.insert(&coordinate, Arc::downgrade(&arc_chunk));
				(true, arc_chunk)
			}
		};

		log::info!(target: "chunk-loading", "Finished loading chunk {}", coordinate);
		ticket.lock().unwrap().set_chunk(arc_chunk.clone());
		(freshly_loaded, arc_chunk)
	}
}
