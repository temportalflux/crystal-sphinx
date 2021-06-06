use crate::engine::ui::*;
use serde::{Deserialize, Serialize};

#[derive(PropsData, Debug, Clone, Serialize, Deserialize)]
pub struct Props {
	pub is_enabled: bool,
}

impl Default for Props {
	fn default() -> Self {
		Self { is_enabled: true }
	}
}
