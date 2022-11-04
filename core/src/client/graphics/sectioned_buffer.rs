use crate::common::utility::VecSectioned;
use engine::channels::mpsc::Sender;
use engine::graphics::{
	alloc::Allocator, buffer::Buffer, command, flags, utility::NamedObject, GpuOpContext,
	GpuOperationBuilder,
};
use std::sync::Mutex;
use std::{
	collections::HashMap,
	hash::Hash,
	ops::Range,
	sync::{Arc, RwLock},
};

/// A buffer of data whose elements are split into sections and referencable by key.
/// Data is preparred in-memory and optimized for minimal-writes-to-GPU,
/// at the cost of additional time inserting/removing elements.
/// Local data is internally-mutable, so users do not need to interface with locking directly.
pub struct SectionedBuffer<S: Eq + Hash + Clone, K: Eq + Hash, V> {
	local_data: RwLock<VecSectioned<S, K, V>>,
	submitted_sections: RwLock<HashMap<S, Range<usize>>>,
	buffer: Mutex<Arc<Buffer>>,
}

impl<S, K, V> SectionedBuffer<S, K, V>
where
	S: Eq + Hash + Clone,
	K: Eq + Hash + Clone,
	V: Sized,
{
	/// Creates a new sectioned buffer, with the size to hold up to `capacity` elements of type `V`.
	/// The buffer will be dynamically up-sized if the contents grow beyond `capacity`.
	pub fn new(name: String, allocator: &Arc<Allocator>, capacity: usize) -> anyhow::Result<Self> {
		let local_data = VecSectioned::with_capacity(capacity);
		let buffer = Buffer::create_gpu(
			name,
			allocator,
			flags::BufferUsage::VERTEX_BUFFER,
			capacity * std::mem::size_of::<V>(),
			None,
			false,
		)?;
		Ok(Self {
			local_data: RwLock::new(local_data),
			submitted_sections: RwLock::new(HashMap::new()),
			buffer: Mutex::new(buffer),
		})
	}

	/// Inserts a value by some key to a section in the buffer.
	#[profiling::function]
	pub fn insert(&self, key: &K, value: V, section: &S) {
		self.local_data.write().unwrap().insert(section, key, value);
	}

	/// Updates a value by some key in the buffer.
	#[profiling::function]
	pub fn update(&self, key: &K, value: V) {
		self.local_data.write().unwrap().update(key, value);
	}

	/// Removes a value by its key from the buffer.
	#[profiling::function]
	pub fn remove(&self, key: &K) -> Option<(S, V)> {
		self.local_data.write().unwrap().remove(key)
	}

	/// Change the section of a value by using its key.
	#[profiling::function]
	pub fn swap(&self, key: &K, new_section: &S) {
		self.local_data.write().unwrap().swap(key, new_section);
	}

	/// Returns the total capacity the GPU buffer must have (in bytes) to suport the local_data.
	fn required_capacity_bytes(&self) -> usize {
		self.local_data.read().unwrap().len() * std::mem::size_of::<V>()
	}

	/// Submits changes in the local data to the GPU buffer.
	///
	/// Returns true if the sections have changed.
	#[profiling::function]
	pub fn submit_changes(
		&self,
		context: &impl GpuOpContext,
		signal_sender: &Sender<Arc<command::Semaphore>>,
	) -> anyhow::Result<bool> {
		// Adjust the allocated GPU buffer if needed
		{
			profiling::scope!("expand-target-buffer");
			// TODO: It may be more performant to expand the buffer logarithmically
			// so it isn't reallocated every time a single element is added.
			let expanded_buffer = {
				let buffer = self.buffer.lock().unwrap();
				buffer.expand(self.required_capacity_bytes())
			};
			if let Some(new_buffer) = expanded_buffer {
				let new_buffer = Arc::new(new_buffer?);
				*self.buffer.lock().unwrap() = new_buffer;
			}
		}

		// Take information about changed indices from local_data.
		let delta = {
			let mut local_data = self.local_data.write().unwrap();
			local_data.take_changed_ranges()
		};
		let (changed_ranges, total_count_changed) = match delta {
			Some(changes) => changes,
			None => return Ok(false),
		};

		let size_of_value = std::mem::size_of::<V>();

		// Create the operation, and staging buffer, that will copy the data to the allocated buffer.
		let mut task = {
			profiling::scope!("prepare-staging-area");
			let operation_name = format!("Write({})", self.buffer.lock().unwrap().name());
			let mut task = GpuOperationBuilder::new(operation_name, context)?.begin()?;
			task.stage_start(total_count_changed * size_of_value)?;
			task
		};

		// Populate the staging buffer and return the list of copy operations
		// (and the full description of all of the sections in the local data).
		let (copy_ranges, section_description) = {
			profiling::scope!("stage-values");
			let local_data = self.local_data.read().unwrap();
			let mut copy_ranges = Vec::with_capacity(changed_ranges.len());
			let mut staging_memory = task.staging_memory()?;
			let mut staging_offset;
			for local_buffer_range in changed_ranges.into_iter() {
				let gpu_byte_start = local_buffer_range.start * size_of_value;
				let gpu_range_in_bytes =
					(local_buffer_range.end - local_buffer_range.start) * size_of_value;
				staging_offset = staging_memory.amount_written();
				staging_memory.write_slice(&local_data.values()[local_buffer_range])?;
				copy_ranges.push(command::CopyBufferRange {
					start_in_src: staging_offset,
					start_in_dst: gpu_byte_start,
					size: gpu_range_in_bytes,
				});
			}
			(copy_ranges, local_data.sections())
		};

		// Run the copy-to-gpu task, sending a gpu-signal to the provided channel
		// so the caller can detect when the operation has completed.
		{
			profiling::scope!("write-stage-to-target");
			let buffer = self.buffer.lock().unwrap();
			task.copy_stage_to_buffer_ranges(&buffer, copy_ranges)
				.send_signal_to(signal_sender)?
				.end()?;
		}

		// Write the updated section description to local data for rendering.
		*self.submitted_sections.write().unwrap() = section_description;

		Ok(true)
	}

	/// Returns a reference to the GPU buffer and the description of its sections.
	/// Use this to determine what the data in the provided buffer is.
	///
	/// It is recommended that callers save the provided arc to some frame-dependent structure to
	/// ensure that it is not dropped while being used by a frame (which can happen if the next call
	/// to [`submit_changes`] results in the buffer being reallocated to fit the updated size).
	pub fn submitted(&self) -> (Arc<Buffer>, HashMap<S, Range<usize>>) {
		let submitted_sections = self.submitted_sections.read().unwrap().clone();
		(self.buffer(), submitted_sections)
	}

	pub fn buffer(&self) -> Arc<Buffer> {
		self.buffer.lock().unwrap().clone()
	}
}
