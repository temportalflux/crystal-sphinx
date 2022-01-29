use crate::common::{
	account::{self, Account},
	utility::DataFile,
};
use engine::utility::Result;
use std::{collections::HashMap, path::PathBuf};

static LOG: &'static str = "account-manager";

/// The registry of all accounts on a client.
/// Is not applicable for querying the accounts that have logged into a game save.
pub struct Manager {
	root: PathBuf,
	accounts: HashMap<account::Id, Account>,
	active_id: Option<account::Id>,
}

impl Default for Manager {
	fn default() -> Self {
		let mut root = std::env::current_dir().unwrap().to_owned();
		root.push("accounts");
		Self {
			root,
			accounts: HashMap::new(),
			active_id: None,
		}
	}
}

impl Manager {
	fn get() -> &'static std::sync::RwLock<Self> {
		use engine::utility::singleton::*;
		static mut INSTANCE: Singleton<Manager> = Singleton::uninit();
		unsafe { INSTANCE.get_or_default() }
	}

	pub fn write() -> std::sync::LockResult<std::sync::RwLockWriteGuard<'static, Self>> {
		Self::get().write()
	}

	pub fn read() -> std::sync::LockResult<std::sync::RwLockReadGuard<'static, Self>> {
		Self::get().read()
	}
}

impl Manager {
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

	pub fn find_id(&self, name: &String) -> Option<account::Id> {
		for (id, account) in self.accounts.iter() {
			if account.display_name() == name {
				return Some(id.clone());
			}
		}
		None
	}

	pub fn create_account(&mut self, name: String) -> Result<account::Id> {
		let account = Account::new_private(&self.root, name)?;
		log::info!(target: LOG, "Created account {}", account);
		account.save(&account.path())?;
		let id = account.id().clone();
		self.accounts.insert(account.id().clone(), account);
		Ok(id)
	}

	pub fn ensure_account(&mut self, name: &String) -> Result<account::Id> {
		match self.find_id(name) {
			Some(account_id) => Ok(account_id),
			None => Ok(self.create_account(name.to_owned())?),
		}
	}

	pub fn login_as(&mut self, id: &account::Id) -> Result<()> {
		if !self.accounts.contains_key(id) {
			log::error!(target: LOG, "No account with id {}", id);
			return Ok(());
		}
		if self.active_id.is_some() {
			self.logout();
		}
		self.active_id = Some(id.clone());
		log::info!(
			target: LOG,
			"Logged in as {}",
			self.active_account().unwrap()
		);
		Ok(())
	}

	pub fn logout(&mut self) {
		if self.active_id.is_some() {
			log::info!(
				target: LOG,
				"Logged out from {}",
				self.active_account().unwrap()
			);
			self.active_id = None;
		}
	}

	pub fn active_account(&self) -> Result<&Account> {
		match &self.active_id {
			Some(id) => Ok(self
				.accounts
				.get(id)
				.ok_or(Error::DoesNotExist(id.clone()))?),
			None => Err(Error::NoAccountLoggedIn)?,
		}
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Client has no account logged in")]
	NoAccountLoggedIn,
	#[error("No account exists with the id({0})")]
	DoesNotExist(String),
}
