use {
	crate::text::*,
	taffy::Size,
	vello::{
		peniko::{self, Brush, Image},
		kurbo::RoundedRectRadii
	}
};

/// A UI node's style properties (in addition to Taffy layout)
#[derive(Debug, Clone)]
pub struct Paint<'buffer> {
	pub background: Brush,
	pub border_color: peniko::Color,
	pub border_radius: RoundedRectRadii,
	pub box_shadow: Option<BoxShadow>,
	pub inner: Option<InnerElement<'buffer>>,
	pub opacity: f32,
	pub z_index: i32,
	pub clip: bool
}

impl Default for Paint<'_> {
	fn default() -> Self {
		Self {
			background: Brush::default(),
			border_color: peniko::Color::TRANSPARENT,
			border_radius: RoundedRectRadii::default(),
			box_shadow: None,
			inner: None,
			opacity: 1.,
			z_index: 0,
			clip: false,
		}
	}
}

#[derive(Debug, Clone)]
pub enum InnerElement<'buffer> {
	Text(TextElement<'buffer>),
	Image(Image)
}

//TODO should this account for available space?
pub fn measure_image(img: &Image, known_dimensions: Size<Option<f32>>) -> Size<f32> {
	match (known_dimensions.width, known_dimensions.height) {
		(Some(width), Some(height)) => Size { width, height },
		(Some(width), None) => Size { width, height: (width / img.width as f32) * img.height as f32 },
		(None, Some(height)) => Size { width: (height / img.height as f32) * img.width as f32, height },
		(None, None) => Size { width: img.width as f32, height: img.height as f32 },
	}
}

/// Box shadow
#[derive(Debug, Clone)]
pub struct BoxShadow {
	pub offset_x: f32,
	pub offset_y: f32,
	pub blur: f32,
	pub spread: f64,
	pub color: peniko::Color
}

const fn as_peniko_color(color: glyphon::Color) -> peniko::Color {
	peniko::Color::from_rgba8(
		((color.0 & 0x00_FF_00_00) >> 16) as u8,
		((color.0 & 0x00_00_FF_00) >> 8) as u8,
		(color.0 & 0x00_00_00_FF) as u8,
		((color.0 & 0xFF_00_00_00) >> 24) as u8
	)
}

fn as_glyphon_color(color: peniko::Color) -> glyphon::Color {
	let color = color.to_rgba8();
	glyphon::Color::rgba(color.r, color.g, color.b, color.a)
}