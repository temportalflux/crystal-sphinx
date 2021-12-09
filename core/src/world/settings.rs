use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Settings {
	#[serde(default = "Settings::default_seed")]
	seed: String,
}

impl Settings {
	fn default_seed() -> String {
		chrono::prelude::Utc::now()
			.format("%Y%m%d%H%M%S")
			.to_string()
	}
}
