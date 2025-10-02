use {
	vello::peniko::{self, Image, Brush},
	glyphon::{Attrs, FontSystem, Buffer, Metrics, Shaping, Wrap},
	taffy::{Size, AvailableSpace},
};

/// A UI node's style properties (in addition to Taffy layout)
#[derive(Debug, Clone, Default)]
pub struct Paint {
	pub background: Brush,
	pub border: Option<Border>,//already exists in taffy::Style
	pub border_radius: BorderRadius,
	pub box_shadow: Option<BoxShadow>,
	pub inner: Option<InnerElement>,
	pub opacity: f32,
	pub z_index: i32,
	pub clip: bool
}

#[derive(Debug, Clone)]
pub enum InnerElement {
	Text(TextElement),
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

#[derive(Debug, Clone)]
pub struct TextElement(pub Buffer);

impl TextElement {
	pub fn new(text: &str, font_system: &mut FontSystem, metrics: Metrics, attrs: &Attrs) -> Self {
		let mut buffer = Buffer::new(font_system, metrics);
		buffer.set_text(font_system, text, attrs, Shaping::Advanced);
		buffer.set_wrap(font_system, Wrap::Word);
		buffer.set_tab_width(font_system, 4);
		Self(buffer)
	}
	
	//TODO should this account for height constraint? I guess this is how text overflow is handled
	pub fn measure(
		&mut self,
		known_dimensions: Size<Option<f32>>,
		available_space: Size<AvailableSpace>,
		font_system: &mut FontSystem,
	) -> Size<f32> {
		// Set width constraint
		let width_constraint = known_dimensions.width.or(match available_space.width {
			AvailableSpace::MinContent => Some(0.0),
			AvailableSpace::MaxContent => None,
			AvailableSpace::Definite(width) => Some(width),
		});
		self.0.set_size(font_system, width_constraint, None);
		
		// Compute layout
		self.0.shape_until_scroll(font_system, false);
		
		// Determine measured size of text
		let (width, total_lines) = self
			.0
			.layout_runs()
			.fold((0.0, 0usize), |(width, total_lines), run| (run.line_w.max(width), total_lines + 1));
		let height = total_lines as f32 * self.0.metrics().line_height;
		
		Size { width, height }
	}
}

/// Border style
#[derive(Debug, Clone)]
pub struct Border {
	pub color: peniko::Color,
	pub width: EdgeSizes
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeSizes {
	pub top: f32,
	pub right: f32,
	pub bottom: f32,
	pub left: f32
}

/// Border radius (rounded corners)
#[derive(Debug, Clone, Copy, Default)]
pub struct BorderRadius {
	pub top_left: f32,
	pub top_right: f32,
	pub bottom_right: f32,
	pub bottom_left: f32
}

/// Box shadow
#[derive(Debug, Clone)]
pub struct BoxShadow {
	pub offset_x: f32,
	pub offset_y: f32,
	pub blur: f32,
	pub spread: f32,
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