use engine::network::mode;
use std::{
	mem::MaybeUninit,
	sync::{Once, RwLock},
};

pub use mode::*;

fn instance() -> &'static RwLock<mode::Set> {
	static mut INSTANCE: (MaybeUninit<RwLock<mode::Set>>, Once) =
		(MaybeUninit::uninit(), Once::new());
	unsafe {
		INSTANCE.1.call_once(|| {
			INSTANCE
				.0
				.as_mut_ptr()
				.write(RwLock::new(mode::Set::empty()))
		});
		&*INSTANCE.0.as_ptr()
	}
}

pub fn set(mode: mode::Set) {
	*instance().write().unwrap() = mode;
}

pub fn get() -> mode::Set {
	instance().read().unwrap().clone()
}
