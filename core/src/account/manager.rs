use super::{Account, LOG};
use crate::engine::utility::VoidResult;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

pub struct Manager {
	root: PathBuf,
	accounts: HashMap<String, Account>,
}

impl Manager {
	pub fn new(parent_dir: &Path) -> Self {
		let mut root = parent_dir.to_owned();
		root.push("accounts");
		Self {
			root,
			accounts: HashMap::new(),
		}
	}

	pub fn scan_accounts(&mut self) -> VoidResult {
		if !self.root.exists() {
			std::fs::create_dir_all(&self.root)?;
		}
		for entry in std::fs::read_dir(&self.root)? {
			let account = Account::load(&entry?.path())?;
			log::info!(target: LOG, "Scanned account {}", account);
			self.accounts.insert(account.id().clone(), account);
		}
		Ok(())
	}

	pub fn create_account(&mut self, account_id: &String) -> VoidResult {
		let account = Account::new(&self.root, account_id);
		log::info!(target: LOG, "Created account {}", account);
		account.save()?;
		self.accounts.insert(account_id.clone(), account);
		Ok(())
	}

	pub fn contains(&self, account_id: &String) -> bool {
		self.accounts.contains_key(account_id)
	}

	pub fn get(&self, account_id: &String) -> Option<&Account> {
		self.accounts.get(account_id)
	}
}
