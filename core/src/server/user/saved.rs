use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct Saved {
	save_location: PathBuf,
}

impl Saved {
	#[profiling::function]
	pub fn load(dir: &Path) -> Result<Self> {
		Ok(Self {
			save_location: dir.to_owned(),
		})
	}

	#[profiling::function]
	pub fn save(&self) -> Result<()> {
		std::fs::create_dir_all(&self.save_location)?;
		Ok(())
	}
}
