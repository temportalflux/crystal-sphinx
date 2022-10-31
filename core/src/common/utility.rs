mod data_file;
pub use data_file::*;

mod multi_hash_map;
pub use multi_hash_map::*;

mod vec_sectioned;
pub use vec_sectioned::*;

pub fn get_named_arg(name: &str) -> Option<u16> {
	std::env::args().find_map(|arg| {
		let prefix = format!("-{}=", name);
		arg.strip_prefix(&prefix)
			.map(|s| s.parse::<u16>().ok())
			.flatten()
	})
}

pub struct ThreadHandle {
	stop_signal: Option<std::sync::Arc<()>>,
	join_handle: Option<std::thread::JoinHandle<()>>,
}
impl ThreadHandle {
	pub fn new(stop_signal: std::sync::Arc<()>, handle: std::thread::JoinHandle<()>) -> Self {
		Self {
			stop_signal: Some(stop_signal),
			join_handle: Some(handle),
		}
	}

	pub fn stop(&mut self) {
		self.stop_signal = None;
	}

	pub fn join(&mut self) {
		if let Some(handle) = self.join_handle.take() {
			let _ = handle.join();
		}
	}
}
impl Drop for ThreadHandle {
	fn drop(&mut self) {
		self.stop();
		self.join();
	}
}
