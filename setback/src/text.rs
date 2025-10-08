use {
	glyphon::{
		fontdb::ID,
		Attrs, Buffer, Cursor, Edit, Editor, FontSystem, LayoutRun, Metrics, Shaping, Wrap
	},
	std::{
		cmp,
		sync::Arc
	},
	taffy::AvailableSpace,
	unicode_segmentation::UnicodeSegmentation,
	vello::{
		kurbo::{Affine, Point, Rect, Size},
		peniko::{self, Color, Fill},
		Glyph, Scene
	}
};

pub struct Font {
	pub id: ID,
	pub blob: peniko::Font
}

pub struct Fonts {
	pub georgia: Font,
	pub georgia_bold: Font,
	pub georgia_italic: Font,
	pub times_new_roman: Font
}

impl Fonts {
	pub fn get_font(&self, id: ID) -> peniko::Font {
		match id {
			id if id == self.georgia.id => self.georgia.blob.clone(),
			id if id == self.georgia_bold.id => self.georgia_bold.blob.clone(),
			id if id == self.georgia_italic.id => self.georgia_italic.blob.clone(),
			id if id == self.times_new_roman.id => self.times_new_roman.blob.clone(),
			_ => unreachable!("Font not loaded")
		}
	}
}

pub struct TextSettings<'a> {
	text: &'a str,
	font: ID,
	color: Color,
	font_size: f32,
	line_height: f32
}

pub struct EditorSettings<'a> {
	text_settings: TextSettings<'a>,
	cursor_color: Color,
	selection_color: Color,
	selected_text_color: Color,
}

#[derive(Debug, Clone)]
enum InnerText<'buffer> {
	Buffer(Buffer),
	Editor(Editor<'buffer>)
}

#[derive(Debug, Clone)]
pub struct TextElement<'buffer> {
	inner_text: InnerText<'buffer>,
	buffer_glyphs: Option<BufferGlyphs>,
	color: Color,
	cursor_color: Color,
	selection_color: Color,
	selected_text_color: Color,
}

impl<'buffer> TextElement<'buffer> {
	pub fn new_text(settings: TextSettings, font_system: &mut FontSystem) -> Self {
		let metrics = Metrics::new(settings.font_size, settings.line_height);
		let mut buffer = Buffer::new(font_system, metrics);
		buffer.set_metrics(font_system, metrics);
		
		// Attributes indicate what font to choose
		let attrs = Attrs::new();
		
		buffer.set_text(font_system, settings.text, &attrs, Shaping::Advanced);
		buffer.set_wrap(font_system, Wrap::Word);
		buffer.set_tab_width(font_system, 4);
		Self {
			inner_text: InnerText::Buffer(buffer),
			buffer_glyphs: None,
			color: settings.color,
			cursor_color: Color::TRANSPARENT,
			selection_color: Color::TRANSPARENT,
			selected_text_color: Color::TRANSPARENT
		}
	}
	
	pub fn new_editor(settings: EditorSettings, font_system: &mut FontSystem) -> Self {
		let metrics = Metrics::new(settings.text_settings.font_size, settings.text_settings.line_height);
		let buffer = Buffer::new(font_system, metrics);
		let mut editor = Editor::new(buffer);
		editor.with_buffer_mut(|buffer| {
			buffer.set_metrics(font_system, metrics);
			
			// Attributes indicate what font to choose
			let attrs = Attrs::new();
			
			buffer.set_text(font_system, settings.text_settings.text, &attrs, Shaping::Advanced);
			buffer.set_wrap(font_system, Wrap::Word);
			buffer.set_tab_width(font_system, 4);
		});
		editor.set_cursor(Cursor::new(0, 0));
		Self {
			inner_text: InnerText::Editor(editor),
			buffer_glyphs: None,
			color: settings.text_settings.color,
			cursor_color: settings.cursor_color,
			selection_color: settings.selection_color,
			selected_text_color: settings.selected_text_color
		}
	}
	
	pub fn measure(
		&mut self,
		known_dimensions: taffy::Size<Option<f32>>,
		available_space: taffy::Size<AvailableSpace>,
		font_system: &mut FontSystem,
	) -> taffy::Size<f32> {
		let width_constraint = known_dimensions.width.or(match available_space.width {
			AvailableSpace::MinContent => Some(0.0),
			AvailableSpace::MaxContent => None,
			AvailableSpace::Definite(width) => Some(width)
		});
		let height_constraint = known_dimensions.height.or(match available_space.height {
			AvailableSpace::MinContent => Some(0.0),
			AvailableSpace::MaxContent => None,
			AvailableSpace::Definite(height) => Some(height)
		});
		let (width, total_lines, line_height) = match &mut self.inner_text {
			InnerText::Buffer(buffer) => {
				buffer.set_size(font_system, width_constraint, height_constraint);
				buffer.shape_until_scroll(font_system, false);
				self.buffer_glyphs = Some(create_glyphs(buffer, self.color, None));
				let d = buffer
					.layout_runs()
					.fold((0.0, 0usize), |(width, total_lines), run| (run.line_w.max(width), total_lines + 1));
				(d.0, d.1, buffer.metrics().line_height)
			}
			InnerText::Editor(editor) => {
				let d = editor.with_buffer_mut(|buffer| {
					buffer.set_size(font_system, width_constraint, height_constraint);
					let d = buffer
						.layout_runs()
						.fold((0.0, 0usize), |(width, total_lines), run| (run.line_w.max(width), total_lines + 1));
					(d.0, d.1, buffer.metrics().line_height)
				});
				editor.shape_as_needed(font_system, false);
				self.buffer_glyphs = Some(create_glyphs_for_editor(
					editor,
					self.color,
					self.cursor_color,
					self.selection_color,
					self.selected_text_color
				));
				d
			}
		};
		
		// Determine measured size of text
		let height = total_lines as f32 * line_height;
		
		taffy::Size { width, height }
	}
	
	pub fn draw(&self, scene: &mut Scene, fonts: &Fonts) {
		let text_transform = Affine::translate((500.0, 300.0));
		let buffer_glyphs = self.buffer_glyphs.as_ref().unwrap();
		
		// Draw the Glyphs
		for buffer_line in &buffer_glyphs.buffer_lines {
			for glyph_highlight in &buffer_line.glyph_highlights {
				scene.fill(
					Fill::NonZero,
					text_transform,
					buffer_glyphs.glyph_highlight_color,
					None,
					glyph_highlight,
				);
			}
			
			if let Some(cursor) = &buffer_line.cursor {
				scene.fill(
					Fill::NonZero,
					text_transform,
					buffer_glyphs.cursor_color,
					None,
					cursor,
				);
			}
			
			for glyph_run in &buffer_line.glyph_runs {
				let font = fonts.get_font(glyph_run.font);
				let glyph_color = glyph_run.glyph_color;
				let glyphs = glyph_run.glyphs.clone();
				scene
					.draw_glyphs(&font)
					.font_size(buffer_glyphs.font_size)
					.brush(glyph_color)
					.transform(text_transform)
					.draw(Fill::NonZero, glyphs.into_iter());
			}
		}
	}
}

pub struct CosmicFontBlobAdapter {
	pub font: Arc<glyphon::Font>,
}

impl AsRef<[u8]> for CosmicFontBlobAdapter {
	fn as_ref(&self) -> &[u8] {
		self.font.data()
	}
}

#[derive(Debug, Clone)]
pub struct BufferGlyphs {
	font_size: f32,
	glyph_highlight_color: Color,
	cursor_color: Color,
	buffer_lines: Vec<BufferLine>,
}

#[derive(Debug, Clone)]
pub struct BufferLine {
	glyph_highlights: Vec<Rect>,
	cursor: Option<Rect>,
	glyph_runs: Vec<BufferGlyphRun>,
}

#[derive(Debug, Clone)]
struct BufferGlyphRun {
	font: ID,
	glyphs: Vec<Glyph>,
	glyph_color: Color,
}

struct EditorInfo {
	cursor_color: Color,
	selection_color: Color,
	selected_text_color: Color,
	selection_bounds: Option<(Cursor, Cursor)>,
	cursor: Cursor,
}

impl EditorInfo {
	fn new(
		editor: &Editor<'_>,
		cursor_color: Color,
		selection_color: Color,
		selected_text_color: Color,
	) -> Self {
		Self {
			cursor_color,
			selection_color,
			selected_text_color,
			selection_bounds: editor.selection_bounds(),
			cursor: editor.cursor(),
		}
	}
}

fn create_glyphs_for_editor(
	editor: &Editor<'_>,
	text_color: Color,
	cursor_color: Color,
	selection_color: Color,
	selected_text_color: Color,
) -> BufferGlyphs {
	editor.with_buffer(|buffer| {
		create_glyphs(
			buffer,
			text_color,
			Some(EditorInfo::new(
				editor,
				cursor_color,
				selection_color,
				selected_text_color,
			)),
		)
	})
}

fn create_glyphs(
	buffer: &Buffer,
	text_color: Color,
	editor_info: Option<EditorInfo>,
) -> BufferGlyphs {
	// Get the laid out glyphs and convert them to Glyphs for vello
	
	let mut last_font: Option<(ID, Color)> = None;
	
	let mut buffer_glyphs = BufferGlyphs {
		font_size: buffer.metrics().font_size,
		glyph_highlight_color: Color::WHITE,
		cursor_color: Color::WHITE,
		buffer_lines: vec![],
	};
	
	if let Some(editor_info) = &editor_info {
		buffer_glyphs.cursor_color = editor_info.cursor_color;
		buffer_glyphs.glyph_highlight_color = editor_info.selection_color;
	}
	
	for layout_run in buffer.layout_runs() {
		let mut current_glyphs: Vec<Glyph> = vec![];
		let line_i = layout_run.line_i;
		let line_y = layout_run.line_y as f64;
		let line_top = layout_run.line_top as f64;
		let line_height = layout_run.line_height as f64;
		
		let mut buffer_line = BufferLine {
			glyph_highlights: vec![],
			cursor: None,
			glyph_runs: vec![],
		};
		
		if let Some(editor_info) = &editor_info {
			// Highlight selection
			if let Some((start, end)) = editor_info.selection_bounds && (line_i >= start.line && line_i <= end.line) {
				let mut range_opt = None;
				for glyph in layout_run.glyphs.iter() {
					// Guess x offset based on characters
					let cluster = &layout_run.text[glyph.start..glyph.end];
					let total = cluster.grapheme_indices(true).count();
					let mut c_x = glyph.x;
					let c_w = glyph.w / total as f32;
					for (i, c) in cluster.grapheme_indices(true) {
						let c_start = glyph.start + i;
						let c_end = glyph.start + i + c.len();
						if (start.line != line_i || c_end > start.index)
							&& (end.line != line_i || c_start < end.index)
						{
							range_opt = match range_opt.take() {
								Some((min, max)) => Some((
									cmp::min(min, c_x as i32),
									cmp::max(max, (c_x + c_w) as i32),
								)),
								None => Some((c_x as i32, (c_x + c_w) as i32)),
							};
						} else if let Some((min, max)) = range_opt.take() {
							buffer_line.glyph_highlights.push(Rect::from_origin_size(
								Point::new(min as f64, line_top),
								Size::new(cmp::max(0, max - min) as f64, line_height),
							));
						}
						c_x += c_w;
					}
				}
				
				if layout_run.glyphs.is_empty() && end.line > line_i {
					// Highlight all internal empty lines
					range_opt = Some((0, buffer.size().0.unwrap_or(0.0) as i32));
				}
				
				if let Some((mut min, mut max)) = range_opt.take() {
					if end.line > line_i {
						// Draw to end of line
						if layout_run.rtl {
							min = 0;
						} else {
							max = buffer.size().0.unwrap_or(0.0) as i32;
						}
					}
					buffer_line.glyph_highlights.push(Rect::from_origin_size(
						Point::new(min as f64, line_top),
						Size::new(cmp::max(0, max - min) as f64, line_height),
					));
				}
			}
			
			// Cursor
			if let Some((x, y)) = cursor_position(&editor_info.cursor, &layout_run) {
				buffer_line.cursor = Some(Rect::from_origin_size(
					Point::new(x as f64, y as f64),
					Size::new(1.0, line_height),
				));
			}
		}
		
		for glyph in layout_run.glyphs {
			let mut glyph_color = match glyph.color_opt {
				Some(color) => Color::from_rgba8(color.r(), color.g(), color.b(), color.a()),
				None => text_color,
			};
			
			if let Some(editor_info) = &editor_info
				&& text_color != editor_info.selected_text_color
				&& let Some((start, end)) = editor_info.selection_bounds && (line_i >= start.line
				&& line_i <= end.line
				&& (start.line != line_i || glyph.end > start.index)
				&& (end.line != line_i || glyph.start < end.index))
			{
				glyph_color = editor_info.selected_text_color;
			}
			
			if let Some((last_font, last_glyph_color)) = last_font && (last_font != glyph.font_id || last_glyph_color != glyph_color) {
				buffer_line.glyph_runs.push(BufferGlyphRun {
					font: last_font,
					glyphs: current_glyphs,
					glyph_color: last_glyph_color,
				});
				current_glyphs = vec![];
			}
			
			last_font = Some((glyph.font_id, glyph_color));
			current_glyphs.push(Glyph {
				x: glyph.x,
				y: glyph.y + line_y as f32,
				id: glyph.glyph_id as u32,
			});
		}
		if !current_glyphs.is_empty() {
			let (last_font, last_color) = last_font.unwrap();
			buffer_line.glyph_runs.push(BufferGlyphRun {
				font: last_font,
				glyphs: current_glyphs,
				glyph_color: last_color,
			});
		}
		
		buffer_glyphs.buffer_lines.push(buffer_line);
	}
	
	buffer_glyphs
}

// Copied directly from https://github.com/pop-os/cosmic-text/blob/58c2ccd1fb3daf0abc792f9dd52b5766b7125ccd/src/edit/editor.rs#L66
fn cursor_position(cursor: &Cursor, run: &LayoutRun<'_>) -> Option<(i32, i32)> {
	let (cursor_glyph, cursor_glyph_offset) = cursor_glyph_opt(cursor, run)?;
	let x = match run.glyphs.get(cursor_glyph) {
		Some(glyph) => {
			// Start of detected glyph
			if glyph.level.is_rtl() {
				(glyph.x + glyph.w - cursor_glyph_offset) as i32
			} else {
				(glyph.x + cursor_glyph_offset) as i32
			}
		}
		None => match run.glyphs.last() {
			Some(glyph) => {
				// End of last glyph
				if glyph.level.is_rtl() {
					glyph.x as i32
				} else {
					(glyph.x + glyph.w) as i32
				}
			}
			None => {
				// Start of empty line
				0
			}
		},
	};
	
	Some((x, run.line_top as i32))
}

// Copied directly from https://github.com/pop-os/cosmic-text/blob/58c2ccd1fb3daf0abc792f9dd52b5766b7125ccd/src/edit/editor.rs#L30
fn cursor_glyph_opt(cursor: &Cursor, run: &LayoutRun<'_>) -> Option<(usize, f32)> {
	if cursor.line == run.line_i {
		for (glyph_i, glyph) in run.glyphs.iter().enumerate() {
			if cursor.index == glyph.start {
				return Some((glyph_i, 0.0));
			} else if cursor.index > glyph.start && cursor.index < glyph.end {
				// Guess x offset based on characters
				let mut before = 0;
				let mut total = 0;
				
				let cluster = &run.text[glyph.start..glyph.end];
				for (i, _) in cluster.grapheme_indices(true) {
					if glyph.start + i < cursor.index {
						before += 1;
					}
					total += 1;
				}
				
				let offset = glyph.w * (before as f32) / (total as f32);
				return Some((glyph_i, offset));
			}
		}
		match run.glyphs.last() {
			Some(glyph) => {
				if cursor.index == glyph.end {
					return Some((run.glyphs.len(), 0.0));
				}
			}
			None => {
				return Some((0, 0.0));
			}
		}
	}
	None
}