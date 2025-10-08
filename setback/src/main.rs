mod app;
mod connection;
mod constants;
mod cursor;
mod game;
mod image;
mod keyboard;
mod main_menu;
mod message;
mod multiplayer;
mod paint;
mod settings;
mod setup;
mod state;
mod text;
mod widgets;

use {
	crate::{
		app::*,
		constants::*,
		cursor::*,
		image::*,
		paint::*,
		state::*,
		text::*
	},
	glyphon::{fontdb::Source, FontSystem, SwashCache},
	keyring::{default::default_credential_builder, set_default_credential_builder},
	std::{
		collections::HashSet,
		fs::{self, File},
		io::{BufReader, BufWriter, Write},
		path::Path,
		sync::{Arc, OnceLock},
		time::{Duration, Instant}
	},
	taffy::{Style, TaffyTree},
	tokio::runtime::Builder,
	tracing_appender::{non_blocking::WorkerGuard, rolling},
	tracing_subscriber::fmt,
	vello::{
		kurbo::{Point, Rect, Size},
		peniko::{self, Blob},
		util::RenderContext
	},
	winit::event_loop::{ControlFlow, EventLoop}
};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

fn main() {
	// set up crash log
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}
	std::panic::set_hook(Box::new(|panic_info| {
		log::error!("Panic - see crash.log");
		let path = Path::new(CRASH_LOG);
		const FS_ERR: &str = "Could create crash log";
		fs::create_dir_all(path.parent().expect(FS_ERR)).expect(FS_ERR);
		let file: File = File::create(path).expect("Could not create crash log file");
		const WRITE_ERR: &str = "Could not write to crash log";
		let mut writer = BufWriter::new(file);
		writeln!(writer, "Time: {}", chrono::Local::now()).expect(WRITE_ERR);
		writeln!(writer, "{panic_info}").expect(WRITE_ERR);
		let backtrace = std::backtrace::Backtrace::capture();
		writeln!(writer, "stack backtrace:\n{backtrace}").expect(WRITE_ERR);
		writeln!(writer, "Process finished with exit code 101").expect(WRITE_ERR);
		writer.flush().expect(WRITE_ERR);
	}));
	
	// load settings
	let settings =
		if let Ok(file) = File::open(SETTINGS_JSON) &&
			let Ok(settings) = serde_json::from_reader(BufReader::new(file)) {
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
		};
	
	// init logger
	
	// let (startup_writer, startup_guard) = tracing_appender::non_blocking(rolling::never(LOG_DIR, "latest_startup.log"));
	// let (setback_writer, setback_guard) = tracing_appender::non_blocking(rolling::never(LOG_DIR, "latest_setback.log"));
	// LOG_GUARDS.set([startup_guard, setback_guard]).unwrap();
	// let startup_layer = fmt::layer()
	// 	.with_timer(fmt::time::SystemTime)
	// 	.with_ansi(false)
	// 	.log_internal_errors(true)
	// 	.with_target(true)
	// 	.with_file(true)
	// 	.with_line_number(true)
	// 	.with_level(true)
	// 	.with_thread_names(true)
	// 	.with_thread_ids(true)
	// 	.pretty()
	// 	.with_writer(startup_writer)
	// 	.with_filter(
	// 		EnvFilter::builder()
	// 			.with_default_directive(tracing::Level::from(settings.log_level).into())
	// 			.parse("wgpu,wgpu_hal,init,wgpu_core")
	// 			.unwrap()
	// 	);
	// let setback_layer = fmt::layer()
	// 	.with_timer(fmt::time::SystemTime)
	// 	.with_ansi(false)
	// 	.log_internal_errors(true)
	// 	.with_target(true)
	// 	.with_file(true)
	// 	.with_line_number(true)
	// 	.with_level(true)
	// 	.with_thread_names(true)
	// 	.with_thread_ids(true)
	// 	.pretty()
	// 	.with_writer(setback_writer)
	// 	.with_filter(
	// 		EnvFilter::builder()
	// 			.with_default_directive(tracing::Level::from(settings.log_level).into())
	// 			.parse("setback,libsetback")
	// 			.unwrap()
	// 	);
	// tracing_subscriber::registry()
	// 	.with(startup_layer)
	// 	.with(setback_layer)
	// 	.init();
	
	let (log_writer, log_guard) = tracing_appender::non_blocking(rolling::never(LOG_DIR, "latest.log"));
	LOG_GUARD.set(log_guard).unwrap();
	fmt::fmt()
		.with_timer(fmt::time::SystemTime)
		.with_ansi(false)
		.log_internal_errors(true)
		.with_target(true)
		.with_file(true)
		.with_line_number(true)
		.with_level(true)
		.with_thread_names(true)
		.with_thread_ids(true)
		.pretty()
		.with_writer(log_writer)
		// .with_max_level(tracing::Level::from(settings.log_level))
		.with_max_level(tracing::Level::TRACE)
		.init();
	log::info!("Initialized logger");
	log::info!("{:#?}", settings);
	
	// sets mock keyring for debug builds
	if cfg!(debug_assertions) {
		set_default_credential_builder(default_credential_builder());
		log::info!("Set mock keyring credential builder for development build");
	}
	
	// create application
	let event_loop = EventLoop::new().unwrap();
	
	let mut ui_tree = TaffyTree::<Paint>::new();
	let root_node = ui_tree.new_leaf(Style::default()).unwrap();
	
	let last_frame_time = Instant::now();
	let next_frame_time = last_frame_time + Duration::from_nanos(1_000_000_000 / settings.max_fps as u64);
	
	let bj_normal_select = custom_cursor_from_bytes(&event_loop, BJ_NORMAL_SELECT);
	
	let mut font_system = FontSystem::new();
	let db = font_system.db_mut();
	let georgia_id = db.load_font_source(Source::Binary(Arc::new(GEORGIA)))[0];
	let georgia_bold_id = db.load_font_source(Source::Binary(Arc::new(GEORGIA_BOLD)))[0];
	let georgia_italic_id = db.load_font_source(Source::Binary(Arc::new(GEORGIA_ITALIC)))[0];
	let times_new_roman_id = db.load_font_source(Source::Binary(Arc::new(TIMES_NEW_ROMAN)))[0];
	let georgia_font = font_system.get_font(georgia_id).unwrap();
	let georgia_bold_font = font_system.get_font(georgia_bold_id).unwrap();
	let georgia_italic_font = font_system.get_font(georgia_italic_id).unwrap();
	let times_new_roman_font = font_system.get_font(times_new_roman_id).unwrap();
	let mut georgia = None;
	let mut georgia_bold = None;
	let mut georgia_italic = None;
	let mut times_new_roman = None;
	font_system
		.db()
		.faces()
		.for_each(|face| {
			if face.id == georgia_id {
				georgia = Some(Font {
					id: face.id,
					blob: peniko::Font::new(Blob::new(Arc::new(CosmicFontBlobAdapter {font: georgia_font.clone()})), face.index)
				});
			} else if face.id == georgia_bold_id {
				georgia_italic = Some(Font {
					id: face.id,
					blob: peniko::Font::new(Blob::new(Arc::new(CosmicFontBlobAdapter {font: georgia_bold_font.clone()})), face.index)
				});
			} else if face.id == georgia_italic_id {
				georgia_bold = Some(Font {
					id: face.id,
					blob: peniko::Font::new(Blob::new(Arc::new(CosmicFontBlobAdapter {font: georgia_italic_font.clone()})), face.index)
				});
			} else if face.id == times_new_roman_id {
				times_new_roman = Some(Font {
					id: face.id,
					blob: peniko::Font::new(Blob::new(Arc::new(CosmicFontBlobAdapter {font: times_new_roman_font.clone()})), face.index)
				});
			}
		});
	
	let mut app = App {
		settings,
		rt: Builder::new_multi_thread()
			.enable_all()
			.build()
			.unwrap(),
		conn_state: ConnState::Singleplayer,
		server: None,
		api: None,
		mouse_pos: None,
		mouse_button_events: Vec::with_capacity(2),
		mouse_pressed_left: false,
		mouse_pressed_right: false,
		cursor_image: CursorImage::Icon(bj_normal_select.clone()),
		scroll_delta: None,
		key_events: Vec::with_capacity(2),
		pressed_keys: HashSet::with_capacity(2),
		ui_tree,
		root_node,
		font_system,
		swash_cache: SwashCache::new(),
		context: RenderContext::new(),
		renderers: Vec::with_capacity(1),
		scene: vello::Scene::new(),
		state: RenderState::Suspended(None),
		next_frame_time,
		last_frame_time,
		scale_factor: 1.,
		bot_icon: Image::load_png(BOT_ICON),//TODO make sure all the image sizes/scales are set correctly
		cards_icon: Image::load_png(CARDS_ICON),
		checkmark: Image::load_png(CHECKMARK),
		dropdown_arrow: Image::load_png(DROPDOWN_ARROW),
		gear_icon: Image::load_png(GEAR_ICON),
		person_icon: Image::load_png(PERSON_ICON),
		volume_high: Image::load_png(VOLUME_HIGH),
		volume_low: Image::load_png(VOLUME_LOW),
		volume_mute: Image::load_png(VOLUME_MUTE),
		x_icon: Image::load_png(X_ICON),
		suit_clubs: Image::load_png(SUIT_CLUBS),
		suit_diamonds: Image::load_png(SUIT_DIAMONDS),
		suit_hearts: Image::load_png(SUIT_HEARTS),
		suit_spades: Image::load_png(SUIT_SPADES),
		card_clubs_2: Image::load_png(CARD_CLUBS_2),
		card_diamonds_2: Image::load_png(CARD_DIAMONDS_2),
		card_hearts_2: Image::load_png(CARD_HEARTS_2),
		card_spades_2: Image::load_png(CARD_SPADES_2),
		card_clubs_3: Image::load_png(CARD_CLUBS_3),
		card_diamonds_3: Image::load_png(CARD_DIAMONDS_3),
		card_hearts_3: Image::load_png(CARD_HEARTS_3),
		card_spades_3: Image::load_png(CARD_SPADES_3),
		card_clubs_4: Image::load_png(CARD_CLUBS_4),
		card_diamonds_4: Image::load_png(CARD_DIAMONDS_4),
		card_hearts_4: Image::load_png(CARD_HEARTS_4),
		card_spades_4: Image::load_png(CARD_SPADES_4),
		card_clubs_5: Image::load_png(CARD_CLUBS_5),
		card_diamonds_5: Image::load_png(CARD_DIAMONDS_5),
		card_hearts_5: Image::load_png(CARD_HEARTS_5),
		card_spades_5: Image::load_png(CARD_SPADES_5),
		card_clubs_6: Image::load_png(CARD_CLUBS_6),
		card_diamonds_6: Image::load_png(CARD_DIAMONDS_6),
		card_hearts_6: Image::load_png(CARD_HEARTS_6),
		card_spades_6: Image::load_png(CARD_SPADES_6),
		card_clubs_7: Image::load_png(CARD_CLUBS_7),
		card_diamonds_7: Image::load_png(CARD_DIAMONDS_7),
		card_hearts_7: Image::load_png(CARD_HEARTS_7),
		card_spades_7: Image::load_png(CARD_SPADES_7),
		card_clubs_8: Image::load_png(CARD_CLUBS_8),
		card_diamonds_8: Image::load_png(CARD_DIAMONDS_8),
		card_hearts_8: Image::load_png(CARD_HEARTS_8),
		card_spades_8: Image::load_png(CARD_SPADES_8),
		card_clubs_9: Image::load_png(CARD_CLUBS_9),
		card_diamonds_9: Image::load_png(CARD_DIAMONDS_9),
		card_hearts_9: Image::load_png(CARD_HEARTS_9),
		card_spades_9: Image::load_png(CARD_SPADES_9),
		card_clubs_10: Image::load_png(CARD_CLUBS_10),
		card_diamonds_10: Image::load_png(CARD_DIAMONDS_10),
		card_hearts_10: Image::load_png(CARD_HEARTS_10),
		card_spades_10: Image::load_png(CARD_SPADES_10),
		card_clubs_jack: Image::load_png(CARD_CLUBS_JACK),
		card_diamonds_jack: Image::load_png(CARD_DIAMONDS_JACK),
		card_hearts_jack: Image::load_png(CARD_HEARTS_JACK),
		card_spades_jack: Image::load_png(CARD_SPADES_JACK),
		card_clubs_queen: Image::load_png(CARD_CLUBS_QUEEN),
		card_diamonds_queen: Image::load_png(CARD_DIAMONDS_QUEEN),
		card_hearts_queen: Image::load_png(CARD_HEARTS_QUEEN),
		card_spades_queen: Image::load_png(CARD_SPADES_QUEEN),
		card_clubs_king: Image::load_png(CARD_CLUBS_KING),
		card_diamonds_king: Image::load_png(CARD_DIAMONDS_KING),
		card_hearts_king: Image::load_png(CARD_HEARTS_KING),
		card_spades_king: Image::load_png(CARD_SPADES_KING),
		card_clubs_ace: Image::load_png(CARD_CLUBS_ACE),
		card_diamonds_ace: Image::load_png(CARD_DIAMONDS_ACE),
		card_hearts_ace: Image::load_png(CARD_HEARTS_ACE),
		card_spades_ace: Image::load_png(CARD_SPADES_ACE),
		bj_normal_select,
		bj_club_select: custom_cursor_from_bytes(&event_loop, BJ_CLUB_SELECT),
		bj_heart_select: custom_cursor_from_bytes(&event_loop, BJ_HEART_SELECT),
		bj_diamond_select: custom_cursor_from_bytes(&event_loop, BJ_DIAMOND_SELECT),
		bj_card_front: custom_cursor_from_bytes(&event_loop, BJ_CARD_FRONT),
		bj_card_back: custom_cursor_from_bytes(&event_loop, BJ_CARD_BACK),
		cursor_atlas: Image::load_png(CURSOR_ATLAS),//TODO does the cursor atlas need to be scaled?
		animated_cursors: {
			//TODO does the cursor atlas need to be scaled?
			let cursor_atlas_json: TextureAtlasJson = serde_json::from_str(CURSOR_ATLAS_JSON).unwrap();
			let mut animations = std::array::from_fn(|_|Vec::new());
			let mut anim_vec: Vec<(AnimatedCursor, usize, Rect)> = cursor_atlas_json
				.frames
				.iter()
				.map(|frame| {
					let (group, num_ext) = frame
						.filename
						.rsplit_once('_')
						.unwrap();
					// frame_num is 1-based, not 0-based
					let frame_num: usize = num_ext
						.strip_suffix(".png")
						.unwrap()
						.parse::<usize>()
						.unwrap();
					let name = match group {
						// Card flips over
						"Blackjack Card Alternate_page" => AnimatedCursor::BjCardAltPage,
						// Card flat spin
						"Blackjack Card Busy_page" => AnimatedCursor::BjCardBusyPage,
						// Card flips over
						"Blackjack Card Busy 2_page" => AnimatedCursor::BjCardBusy2Page,
						// Spade morphs into straight line
						"Blackjack Handwriting_page" => AnimatedCursor::BjHandwritingPage,
						// Rotating heart
						"Blackjack Heart Busy_page" => AnimatedCursor::BjHeartBusyPage,
						// Spade with red and white flashing
						"Blackjack Help Select_page" => AnimatedCursor::BjHelpSelectPage,
						// Bleeding spade
						"Blackjack Help Select 2_page" => AnimatedCursor::BjHelpSelect2Page,
						// Spade with little rotating heart
						"Blackjack Working In Background_page" => AnimatedCursor::BjWorkingInBgPage,
						// Spade morphs into little spade with parallel lines
						"Blackjack Precision Select_page" => AnimatedCursor::BjPrecSelectPage,
						// Spade morphs into standard text-select cursor
						"Blackjack Text Select_page" => AnimatedCursor::BjTextSelectPage,
						_ => unreachable!("Animation doesn't exist: {}", group)
					};
					(name, frame_num, Rect::from_origin_size(
						Point { x: frame.frame.x as f64, y: frame.frame.y as f64 },
						Size { width: frame.frame.w as f64, height: frame.frame.h as f64 }
					))
				}).collect();
			anim_vec.sort_by(|(_,frame_a,_), (_,frame_b,_)| frame_a.cmp(frame_b));
			anim_vec
				.iter()
				.for_each(|(name, _, rect)| {
					animations[*name as usize].push(*rect);
				});
			animations
		},
		loading: images_from_gif_with_size(LOADING, Size::new(1567., 1561.)),
		fonts: Fonts {
			georgia: georgia.unwrap(),
			georgia_bold: georgia_bold.unwrap(),
			georgia_italic: georgia_italic.unwrap(),
			times_new_roman: times_new_roman.unwrap()
		}
	};
	
	log::info!("Running app");
	event_loop.set_control_flow(if app.settings.vsync {
		ControlFlow::Poll
	} else {
		ControlFlow::WaitUntil(next_frame_time)
	});
	app.start();
	event_loop.run_app(&mut app).unwrap();
	log::info!("Exiting app");
}