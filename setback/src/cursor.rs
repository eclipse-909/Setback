use {
	serde::Deserialize,
	winit::{event::ElementState, event_loop::EventLoop, window::CustomCursor},
};

pub enum CursorImage {
	Icon(CustomCursor),
	Animated {
		animated_cursor: AnimatedCursor,
		curr_duration: f32,
		max_duration: f32,
	},
}

pub fn custom_cursor_from_bytes(event_loop: &EventLoop<()>, icon: &[u8]) -> CustomCursor {
	let img = image::load_from_memory_with_format(icon, image::ImageFormat::Png)
		.unwrap()
		.to_rgba8();
	let samples = img.into_flat_samples();
	let (_, w, h) = samples.extents();
	let (w, h) = (w as u16, h as u16);
	let src = CustomCursor::from_rgba(samples.samples, w, h, w / 2, h / 2).unwrap();
	event_loop.create_custom_cursor(src)
}

#[derive(Deserialize)]
pub struct TextureAtlasJson {
	pub frames: Vec<Frame>,
}

#[derive(Deserialize)]
pub struct Frame {
	pub filename: String,
	pub frame: RectData,
}

#[derive(Deserialize)]
pub struct RectData {
	pub x: u32,
	pub y: u32,
	pub w: u32,
	pub h: u32,
}

#[repr(usize)]
#[derive(Clone, Copy)]
pub enum AnimatedCursor {
	/// Card flips over
	BjCardAltPage = 0,
	/// Card flat spin
	BjCardBusyPage = 1,
	/// Card flips over
	BjCardBusy2Page = 2,
	/// Spade morphs into straight line
	BjHandwritingPage = 3,
	/// Rotating heart
	BjHeartBusyPage = 4,
	/// Spade with red and white flashing
	BjHelpSelectPage = 5,
	/// Bleeding spade
	BjHelpSelect2Page = 6,
	/// Spade with little rotating heart
	BjWorkingInBgPage = 7,
	/// Spade morphs into little spade with parallel lines
	BjPrecSelectPage = 8,
	/// Spade morphs into standard text-select cursor
	BjTextSelectPage = 9,
}

pub struct MouseButtonInput {
	pub mouse_button: MouseButton,
	pub state: ElementState,
}

pub enum MouseButton {
	Left,
	Right,
}

impl TryFrom<winit::event::MouseButton> for MouseButton {
	type Error = ();
	fn try_from(button: winit::event::MouseButton) -> Result<Self, Self::Error> {
		match button {
			winit::event::MouseButton::Left => Ok(Self::Left),
			winit::event::MouseButton::Right => Ok(Self::Right),
			_ => Err(()),
		}
	}
}
