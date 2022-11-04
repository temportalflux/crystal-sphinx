pub struct ObjectId {
	pub kind: ObjectKind,
}

impl Into<u128> for ObjectId {
	fn into(self) -> u128 {
		let mut data = 0u128;
		// bits: [0, 1)
		// size: 1 bit
		data |= ((self.kind.id() & /*only allow the first bit of the u8*/1u8) as u128) << 0;
		// bits: [1, 65)
		// size: 64 bits
		data |= (self.kind.data() as u128) << 1;
		data
	}
}

impl From<u128> for ObjectId {
	fn from(data: u128) -> Self {
		let kind_id = ((data >> 0) as u8) & 1u8;
		let kind_data = ((data >> 1) as u64) & u64::MAX;
		Self {
			kind: ObjectKind::from((kind_id, kind_data)),
		}
	}
}

pub enum ObjectKind {
	Entity(hecs::Entity),
	Block,
}

impl ObjectKind {
	fn id(&self) -> u8 {
		match self {
			Self::Entity(_) => 0u8,
			Self::Block => 1u8,
		}
	}

	fn data(&self) -> u64 {
		match self {
			Self::Entity(e) => e.to_bits().get(),
			Self::Block => 0u64,
		}
	}
}

impl From<(u8, u64)> for ObjectKind {
	fn from((id, data): (u8, u64)) -> Self {
		match (id, data) {
			(0u8, data) => Self::Entity(hecs::Entity::from_bits(data).unwrap()),
			(1u8, _) => Self::Block,
			_ => unimplemented!(),
		}
	}
}
