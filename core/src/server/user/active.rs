use crate::{common::account::Account, common::utility::DataFile};
use anyhow::Result;
use std::path::Path;

pub struct Active {
	account: Account,
}

impl Active {
	pub fn new(account: Account) -> Self {
		Self { account }
	}

	#[profiling::function]
	pub fn load(dir: &Path) -> Result<Self> {
		let account = Account::load(&dir)?;
		Ok(Self { account })
	}

	#[profiling::function]
	pub fn save(&self) -> Result<()> {
		self.account.save(&self.account.path())?;
		Ok(())
	}

	pub fn account(&self) -> &Account {
		&self.account
	}

	pub fn account_mut(&mut self) -> &mut Account {
		&mut self.account
	}
}
