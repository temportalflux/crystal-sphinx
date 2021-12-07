use engine::task::{ScheduledTask, Task};
use std::{
	path::PathBuf,
	sync::{Arc, RwLock, Weak},
};
use super::Settings;

pub type ArcLockDatabase = Arc<RwLock<Database>>;

/// The data about a world (its chunks, settings, etc).
/// Exists on the server, does not contain presentational/graphical data.
pub struct Database {
	pub(super) root_path: PathBuf,
	pub(super) settings: Settings,
}

impl Database {
	pub fn new(root_path: PathBuf) -> Self {
		Self { root_path, settings: Settings::default() }
	}

	pub fn settings_path(&self) -> PathBuf {
		let mut path = self.root_path.clone();
		path.push("settings.json");
		path
	}

	pub fn start_loading(arc_world: &ArcLockDatabase) -> Weak<Task> {
		super::task::Load::new(&arc_world).send_to(engine::task::sender())
	}
}
