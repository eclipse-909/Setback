#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod util;

use {
	std::sync::Arc,
	setback::game::GameEventLoop,
	eframe::egui::{self, IconData},
	image::RgbaImage,
	tokio::{
		sync::oneshot::Sender,
		task::JoinHandle
	}
};

fn main() -> eframe::Result {
	eframe::run_native(
		"Setback",
		eframe::NativeOptions {
			viewport: egui::ViewportBuilder {
				icon: match image::open("./game/assets/images/cardsIcon.png") {
					Ok(img) => {
						let icon: RgbaImage = img.to_rgba8();
						let (width, height) = icon.dimensions();
						Some(Arc::new(IconData {rgba: icon.into_raw(), width, height}))
					}
					Err(_) => None,
				},
				..Default::default()
			}
				.with_title("Setback")
				.with_fullscreen(true),
			..Default::default()
		},
		Box::new(|cc| {
			// This gives us image support:
			egui_extras::install_image_loaders(&cc.egui_ctx);
			
			Ok(Box::<SetbackApp>::default())
		}),
	)
}

impl eframe::App for SetbackApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading("Setback");
		});
	}
}

#[derive(Default)]
struct SetbackApp {
	scene_type: SceneType,
	host_type: HostType,
}

///Game state which stores what scene is currently active.
#[derive(Default)]
enum SceneType {
	#[default]
	MainMenu,
	Setup,
	Multiplayer,
	Game,
	Settings
}

///Game state which stores the relation between the player and the type of game they're playing.
#[derive(Default)]
enum HostType {
	///Not in a game.
	#[default]
	None,
	Singleplayer(SPGame),
	///Created a multiplayer game as the host.
	Host,//TODO store Client
	///Joined a multiplayer game as a client.
	Client//TODO store Client
}

enum SPGame {
	Setup(GameEventLoop),
	Playing(JoinHandle<()>, Sender<()>)
}