use {
	crate::{constants::SETTINGS_JSON, state::Level},
	gpui::KeyBinding,
	serde::{Deserialize, Serialize},
	std::{
		fs::File,
		io::{BufReader, BufWriter},
	},
};

pub fn load_settings() -> Settings {
	if let Ok(file) = File::open(SETTINGS_JSON)
		&& let Ok(settings) = serde_json::from_reader(BufReader::new(file))
	{
		settings
	} else {
		let settings = Settings::default();
		if let Ok(file) = File::create(SETTINGS_JSON) {
			if serde_json::to_writer_pretty(BufWriter::new(file), &settings).is_err() {
				log::warn!("Could not write to settings file");
			}
		} else {
			log::warn!("Could not create settings file");
		}
		settings
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
	//video
	pub fullscreen: bool,

	//audio
	pub master_volume: f64,
	pub music_volume: f64,
	pub effects_volume: f64,

	//account
	pub save_login_credentials: bool,

	//logs
	pub setback_log_level: Level,
	pub dep_log_level: Level,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			//video
			fullscreen: !cfg!(debug_assertions),

			//audio
			master_volume: 100.,
			music_volume: 100.,
			effects_volume: 100.,

			//account
			save_login_credentials: true,

			//logs
			setback_log_level: if cfg!(debug_assertions) {
				Level::Debug
			} else {
				Level::Info
			},
			dep_log_level: if cfg!(debug_assertions) {
				Level::Debug
			} else {
				Level::Warn
			},
		}
	}
}

pub struct SettingsUi;
