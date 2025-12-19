use {
	crate::{
		state::Settings,
		constants::SETTINGS_JSON
	},
	std::{fs::File, io::{BufReader, BufWriter}}
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