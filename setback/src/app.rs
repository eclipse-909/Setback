use {
	crate::{
		constants::*,
		cursor::*,
		image::*,
		keyboard::*,
		paint::*,
		state::{self, *},
		text::*
	},
	glyphon::{FontSystem, SwashCache},
	libsetback::*,
	reqwest::Client,
	std::{
		collections::HashSet,
		sync::Arc,
		time::{Duration, Instant}
	},
	taffy::{style_helpers::TaffyMaxContent, Display, NodeId, Rect, Size, Style, TaffyTree},
	tokio::runtime::Runtime,
	vello::{
		kurbo::{self, Affine, RoundedRect, RoundedRectRadii},
		peniko::{Color, Fill, Mix},
		util::RenderContext,
		wgpu,
		AaConfig, AaSupport, Renderer, RendererOptions
	},
	winit::{
		application::ApplicationHandler,
		dpi::PhysicalPosition,
		event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
		event_loop::{ActiveEventLoop, ControlFlow},
		keyboard::{Key, PhysicalKey},
		monitor::{MonitorHandle, VideoModeHandle},
		window::{CustomCursor, Fullscreen, Icon, Window, WindowButtons, WindowId}
	}
};

pub struct App<'buffer> {
	//general app data
	
	pub settings: Settings,
	pub rt: Runtime,
	pub conn_state: ConnState,
	pub server: Option<ServerConnection>,
	pub api: Option<Client>,
	
	//mouse
	
	pub mouse_pos: Option<PhysicalPosition<f64>>,
	pub mouse_button_events: Vec<MouseButtonInput>,
	pub mouse_pressed_left: bool,
	pub mouse_pressed_right: bool,
	pub cursor_image: CursorImage,
	/// x = horizontal, y = vertical
	/// positive values = move right/down or reveal left/up
	pub scroll_delta: Option<PhysicalPosition<f64>>,
	
	//keyboard
	
	pub key_events: Vec<TextInput>,
	pub pressed_keys: HashSet<Keybind>,
	
	//UI
	
	pub ui_tree: TaffyTree<Paint<'buffer>>,
	pub root_node: NodeId,
	
	//font
	
	pub font_system: FontSystem,
	pub swash_cache: SwashCache,
	
	//renderer
	
	pub context: RenderContext,
	pub renderers: Vec<Option<Renderer>>,
	pub scene: vello::Scene,
	pub state: RenderState,
	
	//frame
	
	pub next_frame_time: Instant,
	pub last_frame_time: Instant,
	pub scale_factor: f64,
	
	//assets
	
	//images
	pub bot_icon: Image,
	pub cards_icon: Image,
	pub checkmark: Image,
	pub dropdown_arrow: Image,
	pub gear_icon: Image,
	pub person_icon: Image,
	pub volume_high: Image,
	pub volume_low: Image,
	pub volume_mute: Image,
	pub x_icon: Image,
	
	//suits
	pub suit_clubs: Image,
	pub suit_diamonds: Image,
	pub suit_hearts: Image,
	pub suit_spades: Image,
	
	//cards
	pub card_clubs_2: Image,
	pub card_diamonds_2: Image,
	pub card_hearts_2: Image,
	pub card_spades_2: Image,
	pub card_clubs_3: Image,
	pub card_diamonds_3: Image,
	pub card_hearts_3: Image,
	pub card_spades_3: Image,
	pub card_clubs_4: Image,
	pub card_diamonds_4: Image,
	pub card_hearts_4: Image,
	pub card_spades_4: Image,
	pub card_clubs_5: Image,
	pub card_diamonds_5: Image,
	pub card_hearts_5: Image,
	pub card_spades_5: Image,
	pub card_clubs_6: Image,
	pub card_diamonds_6: Image,
	pub card_hearts_6: Image,
	pub card_spades_6: Image,
	pub card_clubs_7: Image,
	pub card_diamonds_7: Image,
	pub card_hearts_7: Image,
	pub card_spades_7: Image,
	pub card_clubs_8: Image,
	pub card_diamonds_8: Image,
	pub card_hearts_8: Image,
	pub card_spades_8: Image,
	pub card_clubs_9: Image,
	pub card_diamonds_9: Image,
	pub card_hearts_9: Image,
	pub card_spades_9: Image,
	pub card_clubs_10: Image,
	pub card_diamonds_10: Image,
	pub card_hearts_10: Image,
	pub card_spades_10: Image,
	pub card_clubs_jack: Image,
	pub card_diamonds_jack: Image,
	pub card_hearts_jack: Image,
	pub card_spades_jack: Image,
	pub card_clubs_queen: Image,
	pub card_diamonds_queen: Image,
	pub card_hearts_queen: Image,
	pub card_spades_queen: Image,
	pub card_clubs_king: Image,
	pub card_diamonds_king: Image,
	pub card_hearts_king: Image,
	pub card_spades_king: Image,
	pub card_clubs_ace: Image,
	pub card_diamonds_ace: Image,
	pub card_hearts_ace: Image,
	pub card_spades_ace: Image,
	
	//static cursors
	pub bj_normal_select: CustomCursor,
	pub bj_club_select: CustomCursor,
	pub bj_heart_select: CustomCursor,
	pub bj_diamond_select: CustomCursor,
	pub bj_card_front: CustomCursor,
	pub bj_card_back: CustomCursor,
	
	//animated cursor data
	pub cursor_atlas: Image,
	pub animated_cursors: [Vec<kurbo::Rect>;10],
	
	//gifs
	pub loading: Vec<Image>,
	
	//fonts
	pub fonts: Fonts
}

impl<'buffer> App<'buffer> {
	#[inline]
	pub fn start(&mut self) {
		// todo!("handle startup logic")
		let front = self.ui_tree.new_leaf(Style {
			display: Display::Flex,
			size: Size::length(250.),
			margin: Rect::length(25.),
			padding: Rect::length(25.),
			border: Rect::length(25.),
			..Default::default()
		}).unwrap();
		
		self.ui_tree.set_style(self.root_node, Style {
			display: Display::Flex,
			size: Size::length(500.),
			..Default::default()
		}).unwrap();
	}
	
	#[inline]
	fn update(&mut self) {
		let delta_time = Instant::now() - self.last_frame_time;
		let mouse_pos = self.mouse_pos.map(|p| p.to_logical::<f64>(self.scale_factor));
		let scroll_delta = self.scroll_delta.map(|d| d.to_logical::<f64>(self.scale_factor));
		// todo!("handle update logic")
	}
	
	#[inline]
	fn draw_frame(&mut self) {
		//compute layout
		self.ui_tree.compute_layout_with_measure(
			self.root_node,
			Size::MAX_CONTENT,
			|known_dimensions,  available_space, _,  paint, _| {
				if let Size { width: Some(width), height: Some(height) } = known_dimensions {
					return Size { width, height };
				}
				match paint {
					Some(Paint {inner: Some(InnerElement::Text(text)), ..}) => {
						text.measure(known_dimensions, available_space, &mut self.font_system)
					}
					Some(Paint {inner: Some(InnerElement::Image(img)), ..}) => {
						measure_image(img, known_dimensions)
					}
					_ => Size::ZERO
				}
			}
		).unwrap();
		
		//order nodes
		let default_paint = Paint::default();
		let vec_len = self.ui_tree.total_node_count();
		let mut layouts = Vec::with_capacity(vec_len);
		let mut stack = Vec::with_capacity(vec_len);
		stack.push(self.root_node);
		while let Some(node) = stack.pop() {
			layouts.push((self.ui_tree.layout(node).unwrap(), self.ui_tree.get_node_context(node).unwrap_or(&default_paint)));
			let Ok(children) = self.ui_tree.children(node) else {continue};
			for child in children.into_iter().rev() {
				stack.push(child);
			}
		}
		layouts.sort_by(|(layout_a,paint_a), (layout_b,paint_b)|
			(paint_a.z_index, layout_a.order).cmp(&(paint_b.z_index, layout_b.order))
		);
		
		//draw frame
		for (layout, paint) in layouts.iter() {
			let border_rect = kurbo::Rect::new(
				layout.location.x as f64,
				layout.location.y as f64,
				(layout.location.x + layout.size.width + layout.border.left + layout.border.right) as f64,
				(layout.location.x + layout.size.height + layout.border.top + layout.border.bottom) as f64
			);
			//shadow
			if let Some(shadow) = &paint.box_shadow {
				self.scene.draw_blurred_rounded_rect(
					Affine::IDENTITY,
					border_rect.inflate(shadow.spread, shadow.spread).with_origin((layout.location.x + shadow.offset_x, layout.location.y + shadow.offset_y)),
					shadow.color,
					(paint.border_radius.top_left
						+ paint.border_radius.bottom_left
						+ paint.border_radius.top_right
						+ paint.border_radius.bottom_right
					) / 4.,
					shadow.blur as f64,
				);
			}
			
			//border
			if layout.border.top > 0.
				&& layout.border.bottom > 0.
				&& layout.border.left > 0.
				&& layout.border.right > 0.
				&& let Some(r) = paint.border_radius.as_single_radius()
				&& r > 0.
			{
				self.scene.fill(
					Fill::NonZero,
					Affine::IDENTITY,
					&paint.background,
					None,
					&RoundedRect::from_rect(
						border_rect,
						paint.border_radius
					)
				);
			}
			
			//background
			const fn shrink_radius(r: f64, bx: f32, by: f32) -> f64 {
				(r - (bx as f64).max(by as f64)).max(0.0)
			}
			let bg_rect = RoundedRect::new(
				(layout.location.x + layout.border.left) as f64,
				(layout.location.y + layout.border.top) as f64,
				(layout.location.x + layout.size.width + layout.border.left) as f64,
				(layout.location.x + layout.size.height + layout.border.top) as f64,
				RoundedRectRadii {
					top_left: shrink_radius(paint.border_radius.top_left, layout.border.left, layout.border.top),
					top_right: shrink_radius(paint.border_radius.top_right, layout.border.right, layout.border.top),
					bottom_right: shrink_radius(paint.border_radius.bottom_right, layout.border.right, layout.border.bottom),
					bottom_left: shrink_radius(paint.border_radius.bottom_left, layout.border.left, layout.border.bottom),
				}
			);
			self.scene.fill(
				Fill::NonZero,
				Affine::IDENTITY,
				&paint.background,
				None,
				&bg_rect
			);
			
			//inner
			if let Some(inner) = &paint.inner {
				if paint.clip {
					if paint.opacity < 1. {
						self.scene.push_layer(Mix::Normal, paint.opacity, Affine::IDENTITY, &bg_rect);
					} else {
						// self.scene.push_clip_layer(Affine::IDENTITY, &bg_rect);//TODO vello 0.6.0 introduces this change, but dependencies don't compile locally
						self.scene.push_layer(Mix::Clip, 1., Affine::IDENTITY, &bg_rect);
					}
				}
				match inner {
					InnerElement::Image(img) => {
						self.scene.draw_image(img, Affine::IDENTITY);
					}
					InnerElement::Text(text) => {
						text.draw(&mut self.scene, &self.fonts);
					}
				}
				if paint.clip {
					self.scene.pop_layer();
				}
			}
		};
	}
	
	#[inline]
	fn reset_frame_data(&mut self) {
		//use it or lose it
		self.scroll_delta = None;
		self.key_events.clear();
		self.mouse_button_events.clear();
	}
	
	/// Helper function that creates a Winit window and returns it (wrapped in an Arc for sharing between threads)
	#[inline]
	fn create_winit_window(&mut self, event_loop: &ActiveEventLoop) -> Arc<Window> {
		let mut attr = Window::default_attributes()
			// .with_inner_size(LogicalSize::new(1044, 800)) //TODO do I need this?
			.with_title("Setback")
			.with_maximized(true)
			.with_visible(true)//TODO maybe set this to false until window is ready
			.with_window_icon(Some({
				let (width, height) = self.cards_icon.image_img.dimensions();
				Icon::from_rgba(self.cards_icon.image_img.as_raw().clone(), width, height).expect("Failed to open icon")
			}))
			.with_cursor(self.bj_normal_select.clone());
		attr = match self.settings.fullscreen {
			state::Fullscreen::Exclusive => {
				let monitor = match self.settings.monitor {
					Monitor::Current => None,
					Monitor::Primary => event_loop.primary_monitor(),
					Monitor::Index(i) => {
						let mut monitors = event_loop.available_monitors();
						monitors.nth(i)
							.or_else(|| monitors.next())
							.or_else(|| event_loop.primary_monitor())
					}
				};
				let fullscreen = if let Some(monitor) = monitor {
					if let Some(mode) = self.pick_video_mode(&monitor) {
						Fullscreen::Exclusive(mode)
					} else {
						Fullscreen::Borderless(None)
					}
				} else {
					Fullscreen::Borderless(None)
				};
				attr.with_resizable(false)
					.with_enabled_buttons(WindowButtons::empty())
					.with_fullscreen(Some(fullscreen))
					.with_decorations(false)
			}
			state::Fullscreen::Borderless => {
				let monitor = match self.settings.monitor {
					Monitor::Current => None,
					Monitor::Primary => event_loop.primary_monitor(),
					Monitor::Index(i) => event_loop.available_monitors().nth(i)
				};
				attr.with_resizable(false)
					.with_enabled_buttons(WindowButtons::empty())
					.with_fullscreen(Some(Fullscreen::Borderless(monitor)))
					.with_decorations(false)
			}
			state::Fullscreen::Windowed => {
				attr.with_resizable(true)
					.with_enabled_buttons(WindowButtons::all())
					.with_fullscreen(None)
					.with_decorations(true)
			}
		};
		let window = event_loop.create_window(attr).unwrap();
		self.scale_factor = window.scale_factor() * self.settings.dpi_scale;
		Arc::new(window)
	}
	
	#[inline]
	fn pick_video_mode(&self, monitor: &MonitorHandle) -> Option<VideoModeHandle> {
		// Try to find an exact match if any fields are set
		if self.settings.resolution.is_some()
			|| self.settings.refresh_rate_mhz.is_some()
			|| self.settings.bit_depth.is_some()
		{
			let candidate = monitor.video_modes().find(|m| {
				if let Some(res) = self.settings.resolution && m.size() != res {
					return false;
				}
				if let Some(rate) = self.settings.refresh_rate_mhz && m.refresh_rate_millihertz() != rate * 1000 {
					return false;
				}
				if let Some(depth) = self.settings.bit_depth && m.bit_depth() != depth {
					return false;
				}
				true
			});
			if candidate.is_some() {
				return candidate;
			}
		}
		// Fallback: pick the best available
		monitor.video_modes().max_by(|a, b| {
			(
				a.size().width,
				a.size().height,
				a.refresh_rate_millihertz(),
				a.bit_depth(),
			).cmp(&(
				b.size().width,
				b.size().height,
				b.refresh_rate_millihertz(),
				b.bit_depth(),
			))
		})
	}
}

impl<'buffer> ApplicationHandler for App<'buffer> {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let RenderState::Suspended(cached_window) = &mut self.state else {return};
		let window = cached_window
			.take()
			.unwrap_or_else(|| self.create_winit_window(event_loop));
		let size = window.inner_size();
		let surface_future = self.context.create_surface(
			window.clone(),
			size.width,
			size.height,
			if self.settings.vsync {wgpu::PresentMode::AutoVsync} else {wgpu::PresentMode::AutoNoVsync}
		);
		let surface = pollster::block_on(surface_future).expect("Error creating surface");
		self.renderers.resize_with(self.context.devices.len(), || None);
		self.renderers[surface.dev_id].get_or_insert_with(|| Renderer::new(
			&self.context.devices[surface.dev_id].device,
			RendererOptions {
				use_cpu: false,
				antialiasing_support: AaSupport {area: true, msaa8: false, msaa16: false},
				num_init_threads: None,
				pipeline_cache: None //TODO create render cache for vulkan/dx12?
			}
		).expect("Couldn't create renderer"));
		self.state = RenderState::Active {
			surface,
			valid_surface: true,
			window,
		};
	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
		let (surface, valid_surface) = match &mut self.state {
			RenderState::Active {
				surface,
				valid_surface,
				window,
			} if window.id() == window_id => (surface, valid_surface),
			_ => return,
		};
		match event {
			WindowEvent::Resized(size) => {
				if size.width != 0 && size.height != 0 {
					self.context.resize_surface(surface, size.width, size.height);
					*valid_surface = true;
				} else {
					*valid_surface = false;
				}
			}
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::KeyboardInput {event, is_synthetic, ..} => {
				let PhysicalKey::Code(key_code) = event.physical_key else {return};
				if !is_synthetic {
					match event.text {
						Some(text) => self.key_events.push(TextInput::Text(text)),
						None => if let Key::Named(named) = event.logical_key {
							self.key_events.push(TextInput::Named(named))
						}
					}
				}
				match event.state {
					ElementState::Pressed => self.pressed_keys.insert(Keybind {key_code, logical_key: event.logical_key}),
					ElementState::Released => self.pressed_keys.remove(&Keybind {key_code, logical_key: event.logical_key})
				};
			}
			// WindowEvent::Ime(ime) => {todo!("support international keyboards")}
			WindowEvent::CursorMoved {position, ..} => self.mouse_pos = Some(position),
			WindowEvent::CursorLeft {..} => self.mouse_pos = None,
			WindowEvent::MouseWheel {delta, ..} => {
				let p = match delta {
					MouseScrollDelta::LineDelta(horizontal, vertical) => PhysicalPosition {
						x: (horizontal * MOUSE_WHEEL_PIXELS_PER_LINE) as f64,
						y: (vertical * MOUSE_WHEEL_PIXELS_PER_LINE) as f64
					},
					MouseScrollDelta::PixelDelta(p) => p
				};
				if let Some(mut delta) = self.scroll_delta {
					delta.x += p.x;
					delta.y += p.y;
				} else {
					self.scroll_delta = Some(p);
				}
			}
			WindowEvent::MouseInput {state, button, ..} => {
				match button {
					MouseButton::Left => self.mouse_pressed_left = state == ElementState::Pressed,
					MouseButton::Right => self.mouse_pressed_right = state == ElementState::Pressed,
					_ => {}
				}
				if let Ok(mouse_button) = button.try_into() {
					self.mouse_button_events.push(MouseButtonInput {mouse_button, state})
				}
			}
			WindowEvent::ScaleFactorChanged {scale_factor, ..} => self.scale_factor = scale_factor * self.settings.dpi_scale,
			WindowEvent::RedrawRequested => {
				if !*valid_surface {return;}
				self.scene.reset();
				self.update();
				self.reset_frame_data();
				self.draw_frame();
				let RenderState::Active {surface, ..} = &self.state else {return;};
				let device_handle = &self.context.devices[surface.dev_id];
				self.renderers[surface.dev_id]
					.as_mut()
					.unwrap()
					.render_to_texture(
						&device_handle.device,
						&device_handle.queue,
						&self.scene,
						&surface.target_view,
						&vello::RenderParams {
							base_color: Color::from_rgb8(0xff, 0x00, 0x00),
							width: surface.config.width,
							height: surface.config.height,
							antialiasing_method: AaConfig::Area,
						},
					)
					.expect("failed to render to texture");
				let surface_texture = surface
					.surface
					.get_current_texture()
					.expect("failed to get surface texture");
				let mut encoder = device_handle
					.device
					.create_command_encoder(&wgpu::CommandEncoderDescriptor {
						label: Some("Surface Blit"),
					});
				surface.blitter.copy(
					&device_handle.device,
					&mut encoder,
					&surface.target_view,
					&surface_texture
						.texture
						.create_view(&wgpu::TextureViewDescriptor::default()),
				);
				device_handle.queue.submit([encoder.finish()]);
				surface_texture.present();
				device_handle.device.poll(wgpu::Maintain::Poll).panic_on_timeout();
			}
			_ => {}
		}
	}
	
	fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
		let RenderState::Active {window, ..} = &self.state else {return};
		if self.settings.vsync {
			window.request_redraw();
			return;
		}
		let duration = Duration::from_nanos(1_000_000_000 / self.settings.max_fps as u64);
		while Instant::now() >= self.next_frame_time {
			window.request_redraw();
			self.next_frame_time += duration;
		}
		event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_time));
	}
	
	fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
		let RenderState::Active {window, ..} = &self.state else {return};
		self.state = RenderState::Suspended(Some(window.clone()));
	}
}