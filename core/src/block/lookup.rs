use super::Block;
use engine::asset;
use std::{collections::HashMap, sync::Arc};

pub type LookupId = usize;

/// A mapping of [`block id`](asset::Id) to unsized-integer (and back)
/// for serializing block asset ids to save space in memory and in network packets.
#[derive(Default)]
pub struct Lookup {
	ordered_ids: Vec<asset::Id>,
	id_values: HashMap<asset::Id, LookupId>,
}

impl Lookup {
	fn instance() -> &'static mut Option<Arc<Self>> {
		static mut INSTANCE: Option<Arc<Lookup>> = None;
		unsafe { &mut INSTANCE }
	}

	pub fn get() -> Option<&'static Arc<Self>> {
		Self::instance().as_ref()
	}

	pub(crate) fn initialize() {
		// Gather asset ids for all block assets
		let block_ids = {
			let mut block_ids = match asset::Library::read().get_ids_of_type::<Block>() {
				Some(ids) => ids.clone(),
				None => vec![], // No ids were scanned
			};
			block_ids.sort();
			block_ids
		};
		let mut lookup = Self::default();
		for id in block_ids.into_iter() {
			lookup.push(id.clone());
		}
		Self::set(lookup);
	}

	fn set(lookup: Lookup) {
		*Self::instance() = Some(Arc::new(lookup));
	}
}

impl Lookup {
	pub(crate) fn push(&mut self, id: asset::Id) -> LookupId {
		let value = self.ordered_ids.len();
		self.id_values.insert(id.clone(), value);
		self.ordered_ids.push(id);
		value
	}

	pub fn count(&self) -> usize {
		self.ordered_ids.len()
	}

	pub fn lookup_value(id: &asset::Id) -> Option<LookupId> {
		Self::get()
			.map(|lookup| lookup.id_values.get(&id).cloned())
			.flatten()
	}

	pub fn lookup_id(value: LookupId) -> Option<asset::Id> {
		Self::get()
			.map(|lookup| lookup.ordered_ids.get(value).cloned())
			.flatten()
	}
}
