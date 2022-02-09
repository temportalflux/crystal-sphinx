use std::{
	cmp::Eq,
	collections::{hash_map::Keys, HashMap, HashSet},
	hash::Hash,
};

pub struct MultiSet<K: Hash, V: Hash>(HashMap<K, HashSet<V>>);

impl<K, V> Default for MultiSet<K, V>
where
	K: Hash,
	V: Hash,
{
	fn default() -> Self {
		Self(HashMap::new())
	}
}

impl<K, V> Clone for MultiSet<K, V>
where
	K: Hash + Clone,
	V: Hash + Clone,
{
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<K, V> MultiSet<K, V>
where
	K: Hash + Eq,
	V: Hash + Eq,
{
	pub fn from(inner: HashMap<K, HashSet<V>>) -> Self {
		Self(inner)
	}

	pub fn insert(&mut self, key: &K, value: V) -> bool
	where
		K: Clone,
	{
		if !self.0.contains_key(&key) {
			self.0.insert(key.clone(), HashSet::with_capacity(1));
		}
		self.0.get_mut(&key).unwrap().insert(value)
	}

	pub fn insert_all(&mut self, key: &K, values: HashSet<V>) -> usize
	where
		K: Clone,
	{
		if values.is_empty() {
			return 0;
		}
		match self.0.get_mut(&key) {
			Some(set) => {
				let mut count = 0;
				for v in values.into_iter() {
					if set.insert(v) {
						count += 1;
					}
				}
				count
			}
			None => {
				let count = values.len();
				self.0.insert(key.clone(), values);
				count
			}
		}
	}

	pub fn append_keys(&mut self, iter: impl std::iter::Iterator<Item = K>, value: &V)
	where
		K: Clone,
		V: Clone,
	{
		for key in iter {
			self.insert(&key, value.clone());
		}
	}

	pub fn remove_key(&mut self, key: &K) -> Option<HashSet<V>> {
		self.0.remove(&key)
	}

	pub fn remove(&mut self, key: &K, value: &V) -> bool {
		let (success, is_empty) = match self.0.get_mut(&key) {
			Some(set) => {
				let success = set.remove(&value);
				(success, set.is_empty())
			}
			None => (false, false),
		};
		if success && is_empty {
			self.0.remove(&key);
		}
		success
	}

	pub fn remove_value(&mut self, value: &V) -> HashSet<K>
	where
		K: Clone,
	{
		let mut found_in = HashSet::with_capacity(self.0.len());
		for (key, set) in self.0.iter_mut() {
			if set.remove(&value) {
				found_in.insert(key.clone());
			}
		}
		found_in
	}

	pub fn keys<'a>(&'a self) -> Keys<'a, K, HashSet<V>> {
		self.0.keys()
	}

	pub fn into_inner(self) -> HashMap<K, HashSet<V>> {
		self.0
	}

	pub fn difference(&self, other: &Self) -> Self
	where
		K: Clone,
		V: Clone,
	{
		let mut diff = Self::default();
		for (key, set) in self.0.iter() {
			match other.0.get(&key) {
				Some(other_set) => {
					diff.insert_all(key, set.difference(&other_set).cloned().collect());
				}
				None => {
					diff.insert_all(key, set.clone());
				}
			}
		}
		diff
	}

	pub fn intersection(&self, other: &Self) -> Self
	where
		K: Clone,
		V: Clone,
	{
		let mut diff = Self::default();
		for (key, set) in self.0.iter() {
			// If our key is not in other, than the entire subset is not intersected
			if let Some(other_set) = other.0.get(&key) {
				// if a key is in both, then only the intersection of items is relevant
				diff.insert_all(key, set.intersection(&other_set).cloned().collect());
			}
		}
		diff
	}
}
