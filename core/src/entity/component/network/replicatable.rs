use crate::entity::component::{binary, Component, ExtensionRegistration};

/// Trait implemented by components to mark that the component is replicated to relevant connections.
/// Components which are able to be replicated must also implement [`binary serialization`](binary::Serializable).
pub trait Replicatable: binary::Serializable {
	fn on_replication(&mut self, _replicated: &Self, _is_locally_owned: bool) {}
}

pub struct Registration {
	fn_clone_into: Box<dyn Fn(&hecs::EntityBuilder, &mut hecs::EntityBuilder)>,
	fn_on_rep: Box<dyn Fn(&hecs::EntityBuilder, &hecs::EntityRef, bool)>,
}

impl ExtensionRegistration for Registration {
	fn extension_id() -> &'static str
	where
		Self: Sized,
	{
		"replicatable"
	}
}

impl Registration {
	pub fn from<T>() -> Self
	where
		T: Component + Replicatable + Clone,
	{
		Self {
			fn_clone_into: Box::new(|src: &hecs::EntityBuilder, dst: &mut hecs::EntityBuilder| {
				dst.add(src.get::<T>().unwrap().clone());
			}),
			fn_on_rep: Box::new(
				|src: &hecs::EntityBuilder, dst: &hecs::EntityRef, is_locally_owned: bool| {
					let src_c = src.get::<T>().unwrap();
					let mut dst_c = dst.get_mut::<T>().unwrap();
					dst_c.on_replication(src_c, is_locally_owned);
				},
			),
		}
	}

	pub fn clone_into_builder(&self, src: &hecs::EntityBuilder, dst: &mut hecs::EntityBuilder) {
		(self.fn_clone_into)(src, dst)
	}

	pub fn on_replication(
		&self,
		src: &hecs::EntityBuilder,
		dst: &hecs::EntityRef,
		is_locally_owned: bool,
	) {
		(self.fn_on_rep)(src, dst, is_locally_owned)
	}
}
