use super::{Account, Manager, LOG};
use crate::engine::utility::{AnyError, VoidResult};

/// The registry of all accounts on a client.
/// Is not applicable for querying the accounts that have logged into a game save.
pub struct ClientRegistry {
	active_account_id: Option<super::Id>,
	manager: Manager,
}

impl Default for ClientRegistry {
	fn default() -> Self {
		Self {
			manager: Manager::new(&std::env::current_dir().unwrap()),
			active_account_id: None,
		}
	}
}

impl ClientRegistry {
	fn get() -> &'static std::sync::RwLock<Self> {
		use crate::engine::utility::singleton::*;
		static mut INSTANCE: Singleton<ClientRegistry> = Singleton::uninit();
		unsafe { INSTANCE.get_or_default() }
	}

	pub fn write() -> std::sync::LockResult<std::sync::RwLockWriteGuard<'static, Self>> {
		Self::get().write()
	}

	pub fn read() -> std::sync::LockResult<std::sync::RwLockReadGuard<'static, Self>> {
		Self::get().read()
	}
}

impl ClientRegistry {
	pub fn scan_accounts(&mut self) -> VoidResult {
		self.manager.scan_accounts()
	}

	pub fn ensure_account(&mut self, name: &String) -> Result<super::Id, AnyError> {
		match self.manager.find_id(name) {
			Some(account_id) => Ok(account_id),
			None => Ok(self.manager.create_account(name)?),
		}
	}

	pub fn login_as(&mut self, id: &super::Id) -> VoidResult {
		if !self.manager.contains(id) {
			log::error!(target: LOG, "No account with id {}", id);
			return Ok(());
		}
		if self.active_account_id.is_some() {
			self.logout();
		}
		self.active_account_id = Some(id.clone());
		log::info!(
			target: LOG,
			"Logged in as {}",
			self.active_account().unwrap()
		);
		Ok(())
	}

	pub fn logout(&mut self) {
		if self.active_account_id.is_some() {
			log::info!(
				target: LOG,
				"Logged out from {}",
				self.active_account().unwrap()
			);
			self.active_account_id = None;
		}
	}

	pub fn active_account(&self) -> Option<&Account> {
		self.active_account_id
			.as_ref()
			.map(|id| self.manager.get(id))
			.flatten()
	}
}
