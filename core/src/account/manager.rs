use super::{Account, LOG};
use anyhow::Result;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

pub struct Manager {
	root: PathBuf,
	accounts: HashMap<super::Id, Account>,
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

	pub fn scan_accounts(&mut self) -> Result<()> {
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

	pub fn find_id(&self, name: &String) -> Option<super::Id> {
		for (id, account) in self.accounts.iter() {
			if account.display_name() == name {
				return Some(id.clone());
			}
		}
		None
	}

	pub fn create_account(&mut self, name: &String) -> Result<super::Id> {
		let account = Account::new(&self.root, name)?;
		log::info!(target: LOG, "Created account {}", account);
		account.save()?;
		let id = account.id().clone();
		self.accounts.insert(account.id().clone(), account);
		Ok(id)
	}

	pub fn contains(&self, account_id: &super::Id) -> bool {
		self.accounts.contains_key(account_id)
	}

	pub fn get(&self, account_id: &super::Id) -> Option<&Account> {
		self.accounts.get(account_id)
	}
}
