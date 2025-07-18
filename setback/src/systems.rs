use {
	crate::{
		resources::*,
		constants::*
	},
	std::time::Duration,
	bevy::{
		prelude::*,
		winit::cursor::{CursorIcon, CustomCursor, CustomCursorImage}
	}
};

pub fn new_timer() -> Timer {
	Timer::new(Duration::from_secs_f32(1.0 / CURSOR_FPS), TimerMode::Once)
}

pub fn custom_cursor_icon(
	cursor_atlas: &mut CursorAtlas,
	image: Handle<Image>
) -> CursorIcon {
	cursor_atlas.current_animation = "";
	cursor_atlas.current_frame = 0;
	cursor_atlas.frame_timer = new_timer();
	CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
		handle: image,
		..default()
	}))
}

pub fn custom_cursor_anim(
	cursor_atlas: &mut CursorAtlas,
	animation: &'static str
) -> CursorIcon {
	cursor_atlas.current_animation = animation;
	cursor_atlas.current_frame = 0;
	cursor_atlas.frame_timer = new_timer();
	CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
		handle: cursor_atlas.texture.clone(),
		texture_atlas: Some(TextureAtlas {
			layout: cursor_atlas.layout.clone(),
			index: cursor_atlas.animations.get(animation).expect("Invalid animation name")[0]
		}),
		..default()
	}))
}

