use {
	crate::{
		constants::*,
		cursor::*,
		image::*,
		keyboard::*,
		layout::*,
		state::{self, *},
		text::*,
		widgets::*,
	},
	glyphon::SwashCache,
	libsetback::*,
	reqwest::Client,
	std::{
		collections::HashSet,
		sync::Arc,
		time::{Duration, Instant},
	},
	taffy::{NodeId, Size, style_helpers::TaffyMaxContent},
	tokio::runtime::Runtime,
	vello::{
		AaConfig, AaSupport, Renderer, RendererOptions,
		kurbo::{self, Affine, RoundedRect, RoundedRectRadii},
		peniko::{Color, Fill, Mix},
		util::RenderContext,
		wgpu,
	},
	winit::{
		application::ApplicationHandler,
		dpi::{PhysicalPosition, PhysicalSize},
		event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
		event_loop::{ActiveEventLoop, ControlFlow},
		keyboard::{Key, PhysicalKey},
		window::{CustomCursor, Fullscreen, Icon, Window, WindowButtons, WindowId},
	},
};

pub struct App<'scene, 'frame> {
	//general app data
	pub settings: Settings,
	pub rt: Runtime,
	pub conn_state: ConnState,
	pub server: Option<ServerConnection>,
	pub api: Option<Client>,
	pub static_arena: Arena<'static>,
	pub scene_arena: Arena<'scene>,
	pub frame_arena: Arena<'frame>,

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
	pub ui_tree: LayoutTree,
	pub root_node: NodeId,
	pub mouse_scroll_subscribers: Vec<(NodeId, ScrollNotification)>,
	pub mouse_button_subscribers: Vec<(NodeId, MouseButtonNotification)>,
	pub key_subscribers: Vec<(NodeId, KeyNotification)>,

	//font
	pub swash_cache: SwashCache,

	//renderer
	pub context: RenderContext,
	pub renderers: Vec<Option<Renderer>>,
	pub scene: vello::Scene,
	pub state: RenderState,
	pub curr_size: PhysicalSize<u32>,
	pub requested_size: PhysicalSize<u32>,

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
	pub animated_cursors: [Vec<kurbo::Rect>; 10],

	//gifs
	pub loading: Vec<Image>,

	//fonts
	pub fonts: Fonts,
}

impl<'scene, 'frame> ApplicationHandler for App<'scene, 'frame> {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let RenderState::Suspended(cached_window) = &mut self.state else {
			return;
		};
		let window = cached_window.take().unwrap_or_else(|| {
			let mut attr = Window::default_attributes()
				.with_title("Setback")
				.with_maximized(true)
				.with_visible(true) //TODO maybe set this to false until window is ready
				.with_window_icon(Some({
					let (width, height) = self.cards_icon.image_img.dimensions();
					Icon::from_rgba(self.cards_icon.image_img.as_raw().clone(), width, height)
						.expect("Failed to open icon")
				}))
				.with_cursor(self.bj_normal_select.clone());
			attr = match self.settings.fullscreen {
				state::Fullscreen::Exclusive => {
					let monitor = match self.settings.monitor {
						Monitor::Current => None,
						Monitor::Primary => event_loop.primary_monitor(),
						Monitor::Index(i) => {
							let mut monitors = event_loop.available_monitors();
							monitors
								.nth(i)
								.or_else(|| monitors.next())
								.or_else(|| event_loop.primary_monitor())
						}
					};

					let fullscreen = if let Some(monitor) = monitor {
						if let Some(mode) = 'video_mode_binding: {
							// Try to find an exact match if any fields are set
							if self.settings.resolution.is_some()
								|| self.settings.refresh_rate_mhz.is_some()
								|| self.settings.bit_depth.is_some()
							{
								let candidate = monitor.video_modes().find(|m| {
									if let Some(res) = self.settings.resolution
										&& m.size() != res
									{
										return false;
									}
									if let Some(rate) = self.settings.refresh_rate_mhz
										&& m.refresh_rate_millihertz() != rate * 1000
									{
										return false;
									}
									if let Some(depth) = self.settings.bit_depth
										&& m.bit_depth() != depth
									{
										return false;
									}
									true
								});
								if candidate.is_some() {
									break 'video_mode_binding candidate;
								}
							}
							// Fallback: pick the best available
							monitor.video_modes().max_by(|a, b| {
								(
									a.size().width,
									a.size().height,
									a.refresh_rate_millihertz(),
									a.bit_depth(),
								)
									.cmp(&(
										b.size().width,
										b.size().height,
										b.refresh_rate_millihertz(),
										b.bit_depth(),
									))
							})
						} {
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
						Monitor::Index(i) => event_loop.available_monitors().nth(i),
					};
					attr.with_resizable(false)
						.with_enabled_buttons(WindowButtons::empty())
						.with_fullscreen(Some(Fullscreen::Borderless(monitor)))
						.with_decorations(false)
				}
				state::Fullscreen::Windowed => attr
					.with_resizable(true)
					.with_enabled_buttons(WindowButtons::all())
					.with_fullscreen(None)
					.with_decorations(true),
			};
			let window = event_loop.create_window(attr).unwrap();
			self.scale_factor = window.scale_factor() * self.settings.dpi_scale;
			Arc::new(window)
		});
		let size = window.inner_size();
		let surface_future = self.context.create_surface(
			window.clone(),
			size.width,
			size.height,
			if self.settings.vsync {
				wgpu::PresentMode::AutoVsync
			} else {
				wgpu::PresentMode::AutoNoVsync
			},
		);
		let surface = pollster::block_on(surface_future).expect("Error creating surface");
		self.renderers
			.resize_with(self.context.devices.len(), || None);
		self.renderers[surface.dev_id].get_or_insert_with(|| {
			Renderer::new(
				&self.context.devices[surface.dev_id].device,
				RendererOptions {
					use_cpu: false,
					antialiasing_support: AaSupport {
						area: true,
						msaa8: false,
						msaa16: false,
					},
					num_init_threads: None,
					pipeline_cache: None, //TODO create render cache for vulkan/dx12?
				},
			)
			.expect("Couldn't create renderer")
		});
		self.state = RenderState::Active {
			surface,
			valid_surface: true,
			window,
		};
	}

	fn window_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		window_id: WindowId,
		event: WindowEvent,
	) {
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
					self.requested_size = size;
					*valid_surface = true;
				} else {
					*valid_surface = false;
				}
			}
			WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
			WindowEvent::KeyboardInput {
				event,
				is_synthetic,
				..
			} => {
				let PhysicalKey::Code(key_code) = event.physical_key else {
					return;
				};
				if !is_synthetic {
					match event.text {
						Some(text) => self.key_events.push(TextInput::Text(text)),
						None => {
							if let Key::Named(named) = event.logical_key {
								self.key_events.push(TextInput::Named(named))
							}
						}
					}
				}
				match event.state {
					ElementState::Pressed => self.pressed_keys.insert(Keybind {
						key_code,
						logical_key: event.logical_key,
					}),
					ElementState::Released => self.pressed_keys.remove(&Keybind {
						key_code,
						logical_key: event.logical_key,
					}),
				};
			}
			// WindowEvent::Ime(ime) => {todo!("support international keyboards")}
			WindowEvent::CursorMoved { position, .. } => self.mouse_pos = Some(position),
			WindowEvent::CursorLeft { .. } => self.mouse_pos = None,
			WindowEvent::MouseWheel { delta, .. } => {
				let p = match delta {
					MouseScrollDelta::LineDelta(horizontal, vertical) => PhysicalPosition {
						x: (horizontal * MOUSE_WHEEL_PIXELS_PER_LINE) as f64,
						y: (vertical * MOUSE_WHEEL_PIXELS_PER_LINE) as f64,
					},
					MouseScrollDelta::PixelDelta(p) => p,
				};
				if let Some(mut delta) = self.scroll_delta {
					delta.x += p.x;
					delta.y += p.y;
				} else {
					self.scroll_delta = Some(p);
				}
			}
			WindowEvent::MouseInput { state, button, .. } => {
				match button {
					MouseButton::Left => self.mouse_pressed_left = state == ElementState::Pressed,
					MouseButton::Right => self.mouse_pressed_right = state == ElementState::Pressed,
					_ => {}
				}
				if let Ok(mouse_button) = button.try_into() {
					self.mouse_button_events.push(MouseButtonInput {
						mouse_button,
						state,
					})
				}
			}
			WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
				self.scale_factor = scale_factor * self.settings.dpi_scale
			}
			WindowEvent::RedrawRequested => {
				if !*valid_surface {
					return;
				}
				// resize if needed
				if self.curr_size != self.requested_size {
					self.curr_size = self.requested_size;
					self.context.resize_surface(
						surface,
						self.requested_size.width,
						self.requested_size.height,
					);
				}

				// clear scene
				// TODO maybe the scene can be cached if there is no input or changes?
				self.scene.reset();

				// compute layout
				let nodes = self
					.ui_tree
					.compute_layout(self.root_node, Size::MAX_CONTENT);

				// run frame update logic
				let delta_time = Instant::now() - self.last_frame_time;
				let mouse_pos = self
					.mouse_pos
					.map(|p| p.to_logical::<f64>(self.scale_factor));
				let scroll_delta = self
					.scroll_delta
					.map(|d| d.to_logical::<f64>(self.scale_factor));
				// todo!("run update logic")
				// TODO handle cursor hover here
				// Use subscribers to handle input

				// reset frame-specific data
				self.scroll_delta = None;
				self.key_events.clear();
				self.mouse_button_events.clear();

				// draw scene
				for node in nodes.into_iter() {
					let node = Node::from_id(*node);
					let layout = &node.final_layout;
					let style = &node.style;

					let border_rect = kurbo::Rect::new(
						layout.location.x as f64,
						layout.location.y as f64,
						(layout.location.x
							+ layout.size.width + layout.border.left
							+ layout.border.right) as f64,
						(layout.location.y
							+ layout.size.height + layout.border.top
							+ layout.border.bottom) as f64,
					);
					let border_radius = style.border_radius;

					//shadow
					if let Some(shadow) = &style.box_shadow {
						let shadow_spread = shadow.spread.get();
						self.scene.draw_blurred_rounded_rect(
							Affine::IDENTITY,
							border_rect
								.inflate(shadow_spread, shadow_spread)
								.with_origin((
									layout.location.x + shadow.offset_x.get(),
									layout.location.y + shadow.offset_y.get(),
								)),
							shadow.color.get(),
							(border_radius.top_left
								+ border_radius.bottom_left
								+ border_radius.top_right
								+ border_radius.bottom_right)
								/ 4.,
							shadow.blur.get() as f64,
						);
					}

					//border
					if layout.border.top > 0.
						&& layout.border.bottom > 0.
						&& layout.border.left > 0.
						&& layout.border.right > 0.
						&& let Some(r) = border_radius.as_single_radius()
						&& r > 0.
					{
						self.scene.fill(
							Fill::NonZero,
							Affine::IDENTITY,
							style.border_color,
							None,
							&RoundedRect::from_rect(border_rect, border_radius),
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
						(layout.location.y + layout.size.height + layout.border.top) as f64,
						RoundedRectRadii {
							top_left: shrink_radius(
								border_radius.top_left,
								layout.border.left,
								layout.border.top,
							),
							top_right: shrink_radius(
								border_radius.top_right,
								layout.border.right,
								layout.border.top,
							),
							bottom_right: shrink_radius(
								border_radius.bottom_right,
								layout.border.right,
								layout.border.bottom,
							),
							bottom_left: shrink_radius(
								border_radius.bottom_left,
								layout.border.left,
								layout.border.bottom,
							),
						},
					);
					self.scene.fill(
						Fill::NonZero,
						Affine::IDENTITY,
						&style.background,
						None,
						&bg_rect,
					);

					//inner
					let opacity = style.opacity;
					match &node.inner {
						InnerElement::Text(text) => {
							if style.clip {
								if opacity < 1. {
									self.scene.push_layer(
										Mix::Normal,
										opacity,
										Affine::IDENTITY,
										&bg_rect,
									);
								} else {
									// self.scene.push_clip_layer(Affine::IDENTITY, &bg_rect);//TODO vello 0.6.0 introduces this change, but dependencies don't compile locally
									self.scene.push_layer(
										Mix::Clip,
										1.,
										Affine::IDENTITY,
										&bg_rect,
									);
								}
							}
							text.draw(&mut self.scene, &self.fonts);
							if style.clip {
								self.scene.pop_layer();
							}
						}
						InnerElement::Image(img) => {
							if style.clip {
								if opacity < 1. {
									self.scene.push_layer(
										Mix::Normal,
										opacity,
										Affine::IDENTITY,
										&bg_rect,
									);
								} else {
									// self.scene.push_clip_layer(Affine::IDENTITY, &bg_rect);//TODO vello 0.6.0 introduces this change, but dependencies don't compile locally
									self.scene.push_layer(
										Mix::Clip,
										1.,
										Affine::IDENTITY,
										&bg_rect,
									);
								}
							}
							self.scene.draw_image(img, Affine::IDENTITY);
							if style.clip {
								self.scene.pop_layer();
							}
						}
						InnerElement::Children(_) | InnerElement::None => {}
					}
				}

				// render scene
				let RenderState::Active { surface, .. } = &self.state else {
					return;
				};
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
							base_color: Color::from_rgb8(0x00, 0x00, 0x00),
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
				let mut encoder =
					device_handle
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
				device_handle
					.device
					.poll(wgpu::Maintain::Poll)
					.panic_on_timeout();
			}
			_ => {}
		}
	}

	fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
		let RenderState::Active { window, .. } = &self.state else {
			return;
		};
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
		let RenderState::Active { window, .. } = &self.state else {
			return;
		};
		self.state = RenderState::Suspended(Some(window.clone()));
	}
}
