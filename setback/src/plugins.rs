use {
	crate::{
		constants::*,
		models::*,
		resources::*,
		components::*,
	},
	bevy::{
		prelude::*,
		winit::cursor::{CursorIcon, CustomCursor}
	},
	bevy_inspector_egui::{
		bevy_egui::EguiPlugin,
		quick::WorldInspectorPlugin
	},
	bevy_ui_text_input::{TextInputPlugin, TextSubmitEvent},
	std::{
		collections::HashMap,
		task::{Context, Poll}
	},
	futures::StreamExt
};

/// Adds egui inspector. This will mess up the cursor because egui overwrites the cursor image on every frame..
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
	fn build(&self, app: &mut App) {
		if cfg!(debug_assertions) {
			app
				.add_plugins(EguiPlugin::default())
        		.add_plugins(WorldInspectorPlugin::new());
		}
	}
}

/// Adds custom cursors
pub struct CursorIconPlugin;

impl Plugin for CursorIconPlugin {
	fn build(&self, app: &mut App) {
		app
			.init_resource::<CursorAtlas>()
			.add_systems(Startup, init_cursor_icon)
			.add_systems(Update, animate_cursor);
	}
}

fn init_cursor_icon(
	window: Single<Entity, With<Window>>,
	asset_server: Res<AssetServer>,
	mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>
) {
	// Load and parse the JSON
    let json_str: String = std::fs::read_to_string("setback/assets/cursors/cursor_atlas.json").unwrap();
    let parsed: TextureAtlasJson = serde_json::from_str(&json_str).unwrap();

    // Create atlas layout with full size
    let texture: Handle<Image> = asset_server.load("cursors/cursor_atlas.png");
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(parsed.meta.size.w, parsed.meta.size.h));
    let mut animations: HashMap<String, Vec<usize>> = HashMap::new();
    for frame in parsed.frames {
    	let urect = URect::new(
     		frame.frame.x,
       		frame.frame.y,
        	frame.frame.x + frame.frame.w,
      		frame.frame.y + frame.frame.h
     	);
     	// let index: usize = layout
      // 		.textures.iter()
      //   	.position(|element| {
	     //    	element.min == urect.min && element.max == urect.max
      //    	})
      //    	.unwrap_or(layout.add_texture(urect));
      	let index = layout.add_texture(urect);
   		// let index: usize = layout.add_texture();
      	static ERROR: &str = "Cursor texture file name is invalid";
     	let (group, num_ext) = frame
      		.filename
        	.rsplit_once('_')
         	.expect(ERROR);
      	// frame_num is 1-based, not 0-based
      	let frame_num: usize = num_ext
       		.strip_suffix(".png")
         	.expect(ERROR)
      		.parse::<usize>()
      		.expect(ERROR);
       	// animations
        // 	.entry(group.to_string())
        //  	.and_modify(move |frame_nums| {
        //   		if frame_nums.len() < frame_num {
        //     		frame_nums.resize(frame_num, Default::default());
        //     	}
        //     	frame_nums[frame_num - 1] = index;
        //   	})
        //  	.or_insert({
        //   		let mut frame_nums = vec![Default::default(); frame_num];
        //     	frame_nums[frame_num - 1] = index;
        //     	frame_nums
        //   	});
        let key = group.to_string();
        if let Some(frame_nums) = animations.get_mut(&key) {
      		if frame_nums.len() < frame_num {
      			frame_nums.resize(frame_num, Default::default());
           	}
           	frame_nums[frame_num - 1] = index;
        } else {
   	  		let mut frame_nums = vec![Default::default(); frame_num];
           	frame_nums[frame_num - 1] = index;
           	animations.insert(key, frame_nums);
        }
    }
    let layout_handle: Handle<TextureAtlasLayout> = texture_atlases.add(layout);

    // Set default cursor
    let image = asset_server.load(BJ_NORMAL_SELECT);
    let mut cursor_atlas = CursorAtlas {
    	texture,
        layout: layout_handle,
        animations,
        frame_timer: new_timer(),
        current_animation: "",
        current_frame: 0
    };
    commands
        .entity(*window)
        .insert(custom_cursor_icon(&mut cursor_atlas, image));

    // Create cursor atlas resource
    commands.insert_resource(cursor_atlas);
}

fn animate_cursor(
	time: Res<Time>,
	mut cursor_icon: Query<&mut CursorIcon>,
	mut cursor_atlas: ResMut<CursorAtlas>
) {
	let mut cursor_icon = cursor_icon
	    .single_mut()
	    .expect(CURSOR_ERROR);
	let CursorIcon::Custom(CustomCursor::Image(ref mut image)) = *cursor_icon else {
		if cfg!(debug_assertions) {
			// egui inspector sets the cursor to a system cursor
			return
		} else {
			unreachable!("CursorIcon should be Custom with CustomCursor that should be Image")
		}
	};
	cursor_atlas.frame_timer.tick(time.delta());
	if !cursor_atlas.frame_timer.finished() {return}
	let Some(frames) = cursor_atlas.animations.get(cursor_atlas.current_animation) else {return};
	let mut current_frame = cursor_atlas.current_frame + 1;
	if current_frame >= frames.len() {
		current_frame = 0;
	}
	let Some(atlas) = image.texture_atlas.as_mut() else {return};
	atlas.index = frames[current_frame];
	cursor_atlas.current_frame = current_frame;
	cursor_atlas.frame_timer = new_timer();
}

/// Adds custom cursors
pub struct AsyncTaskPlugin;

impl Plugin for AsyncTaskPlugin {
	fn build(&self, app: &mut App) {
		app
			.init_resource::<TaskSchedule>()
			.add_systems(Update, execute_tasks);
	}
}

fn execute_tasks(mut commands: Commands, mut tasks: ResMut<TaskSchedule>) {
	let waker = futures::task::noop_waker();
	let mut context = Context::from_waker(&waker);
	loop {
		match tasks.0.poll_next_unpin(&mut context) {
			Poll::Ready(Some(mut command_queue)) => commands.append(&mut command_queue),
			Poll::Ready(None) => return,
			Poll::Pending => {}
		}
	}
}

pub struct TextInputWrapperPlugin;

impl Plugin for TextInputWrapperPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_plugins(TextInputPlugin)
			.add_systems(Update, update_text_input);
	}
}

fn update_text_input(mut events: EventReader<TextSubmitEvent>, actions: Query<&TextInputOnSubmitAction>, mut commands: Commands) {
	for event in events.read() {
		if let Ok(action) = actions.get(event.entity) && let Some(command) = action.0 {
			commands.queue(command);
		}
	}
}