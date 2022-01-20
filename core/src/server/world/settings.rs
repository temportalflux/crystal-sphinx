use engine::utility::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Settings {
	#[serde(skip)]
	root_path: PathBuf,
	#[serde(default = "Settings::default_seed")]
	seed: String,
}

impl Settings {
	pub fn root_path(&self) -> &Path {
		&self.root_path
	}

	fn default_seed() -> String {
		chrono::prelude::Utc::now()
			.format("%Y%m%d%H%M%S")
			.to_string()
	}

	pub fn seed(&self) -> &String {
		&self.seed
	}
}

impl Settings {
	fn create_path(mut world_root_dir: PathBuf) -> PathBuf {
		world_root_dir.push("settings.json");
		world_root_dir
	}

	pub(super) fn load(world_root_dir: &Path) -> Result<Self> {
		// Ensure the world directory exists
		if !world_root_dir.exists() {
			std::fs::create_dir_all(&world_root_dir)?;
		}

		// Load settings from disk, if it exists
		let settings_path = Self::create_path(world_root_dir.to_owned());
		let mut settings = Self::default();
		if settings_path.exists() {
			let raw = std::fs::read_to_string(&settings_path)?;
			settings = serde_json::from_str(&raw)?;
		}

		settings.root_path = world_root_dir.to_owned();
		if settings.seed.is_empty() {
			settings.seed = Self::default_seed();
		}

		// Auto-save loaded settings to file
		{
			let json = serde_json::to_string_pretty(&settings)?;
			std::fs::write(&settings_path, json)?;
		}

		Ok(settings)
	}
}
