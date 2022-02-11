use anyhow::Result;
use std::path::{Path, PathBuf};

pub trait DataFile {
	fn file_name() -> &'static str;

	fn make_path(parent_dir: &Path) -> PathBuf {
		let mut path = parent_dir.to_owned();
		path.push(Self::file_name());
		path
	}

	fn save(&self, parent_dir: &Path) -> Result<()> {
		if !parent_dir.exists() {
			std::fs::create_dir_all(&parent_dir)?;
		}
		self.save_to(&Self::make_path(&parent_dir))?;
		Ok(())
	}

	fn load(parent_dir: &Path) -> Result<Self>
	where
		Self: Sized,
	{
		Self::load_from(&Self::make_path(&parent_dir))
	}

	fn save_to(&self, file_path: &Path) -> Result<()>;

	fn load_from(file_path: &Path) -> Result<Self>
	where
		Self: Sized;
}
