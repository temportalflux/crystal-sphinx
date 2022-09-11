use crate::entity::component::{self, debug, Registration};
use engine::{asset, ecs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
	asset_id: asset::Id,
}

impl Component {
	pub fn new(id: asset::Id) -> Self {
		Self { asset_id: id }
	}
}

impl ecs::Component for Component {
	type Storage = ecs::VecStorage<Self>;
}

impl component::Component for Component {
	fn unique_id() -> &'static str {
		"crystal_sphinx::model::blender::Component"
	}

	fn display_name() -> &'static str {
		"Blender Model"
	}

	fn registration() -> Registration<Self>
	where
		Self: Sized,
	{
		use debug::Registration as debug;
		Registration::<Self>::default().with_ext(debug::from::<Self>())
	}
}

impl std::fmt::Display for Component {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "blender::Component(asset={})", self.asset_id)
	}
}

impl debug::EguiInformation for Component {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!("Asset Id: {}", self.asset_id));
	}
}
