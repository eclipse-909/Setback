use {
	libsetback::SetbackClient,
	serde::{Deserialize, Serialize},
	tokio::runtime::{Builder, Runtime},
};

pub struct Global {
	pub rt: Runtime,
}

impl gpui::Global for Global {}

impl Default for Global {
	fn default() -> Self {
		Self {
			rt: Builder::new_multi_thread()
				.enable_all()
				.build()
				.expect("Failed to create tokio runtime"),
		}
	}
}

#[derive(Default)]
pub enum Scene {
	#[default]
	MainMenu,
	Multiplayer,
	Settings,
	Setup,
	Game,
}

pub enum ConnState {
	Singleplayer,
	Host,
	Client,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Level {
	Trace,
	Debug,
	Info,
	Warn,
	Error,
}

impl From<tracing::Level> for Level {
	fn from(value: tracing::Level) -> Self {
		match value {
			tracing::Level::TRACE => Self::Trace,
			tracing::Level::DEBUG => Self::Debug,
			tracing::Level::INFO => Self::Info,
			tracing::Level::WARN => Self::Warn,
			tracing::Level::ERROR => Self::Error,
		}
	}
}

impl From<Level> for tracing::Level {
	fn from(value: Level) -> Self {
		match value {
			Level::Trace => Self::TRACE,
			Level::Debug => Self::DEBUG,
			Level::Info => Self::INFO,
			Level::Warn => Self::WARN,
			Level::Error => Self::ERROR,
		}
	}
}
