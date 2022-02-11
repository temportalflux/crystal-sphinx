use enumset::{EnumSet, EnumSetType};
use std::{
	mem::MaybeUninit,
	sync::{Once, RwLock},
};

#[derive(Debug, EnumSetType, Hash)]
pub enum Kind {
	Client,
	Server,
}

pub type Set = EnumSet<Kind>;

impl std::fmt::Display for Kind {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Client => write!(f, "Client"),
			Self::Server => write!(f, "Server"),
		}
	}
}

impl std::ops::Add<Kind> for Kind {
	type Output = Set;
	fn add(self, other: Kind) -> Self::Output {
		let mut set = Set::new();
		set.insert(self);
		set.insert(other);
		set
	}
}

impl std::ops::Add<Set> for Kind {
	type Output = Set;
	fn add(self, other: Set) -> Self::Output {
		let mut set = Set::new();
		set.insert_all(other);
		set.insert(self);
		set
	}
}

// Modes: Client, Server, Client + Server
pub fn all() -> Vec<Set> {
	vec![Kind::Client.into(), Kind::Server.into(), Set::all()]
}

fn instance() -> &'static RwLock<Set> {
	static mut INSTANCE: (MaybeUninit<RwLock<Set>>, Once) = (MaybeUninit::uninit(), Once::new());
	unsafe {
		INSTANCE
			.1
			.call_once(|| INSTANCE.0.as_mut_ptr().write(RwLock::new(Set::empty())));
		&*INSTANCE.0.as_ptr()
	}
}

pub fn set(mode: Set) {
	*instance().write().unwrap() = mode;
}

pub fn get() -> Set {
	instance().read().unwrap().clone()
}
