use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use anyhow::{anyhow, Context, Result};
use bytemuck::{Pod, Zeroable};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use snarkterm_core::{OutputParser, TerminalBuffer};
use std::env;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, RenderPipeline, Surface, SurfaceConfiguration};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GRID_MARGIN_X: f32 = 12.0;
const GRID_MARGIN_Y: f32 = 12.0;
const FONT_SIZE: f32 = 14.0;

const ATLAS_COLS: u32 = 16;
const ATLAS_CELL: u32 = 16;
const ATLAS_SIZE: u32 = ATLAS_COLS * ATLAS_CELL;

const EMBEDDED_FONT: &[u8] = include_bytes!("../../../assets/AdwaitaMono-Regular.ttf");

#[derive(Debug, Default)]
struct Args {
    command: Option<String>,
    no_commentary: bool,
    window: bool,
    help: bool,
    version: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("snarkterm: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8> {
    let args = parse_args(env::args().skip(1))?;

    if args.help {
        print_help();
        return Ok(0);
    }

    if args.version {
        println!("snarkterm {VERSION}");
        return Ok(0);
    }

    if let Some(command) = args.command {
        return run_command(&command, args.no_commentary);
    }

    if args.window {
        run_window()?;
        return Ok(0);
    }

    run_interactive()
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args> {
    let mut parsed = Args::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => parsed.help = true,
            "-V" | "--version" => parsed.version = true,
            "--no-commentary" => parsed.no_commentary = true,
            "--window" => parsed.window = true,
            "-c" | "--command" => {
                let command = args
                    .next()
                    .ok_or_else(|| anyhow!("{arg} requires a command"))?;
                parsed.command = Some(command);
            }
            unsupported if unsupported.starts_with('-') => {
                return Err(anyhow!("unsupported option '{unsupported}'"));
            }
            command => {
                let mut parts = vec![command.to_string()];
                parts.extend(args);
                parsed.command = Some(parts.join(" "));
                break;
            }
        }
    }

    Ok(parsed)
}

fn run_window() -> Result<()> {
    let event_loop = EventLoop::new().context("failed to create window event loop")?;
    let mut app = WindowApp::default();
    event_loop
        .run_app(&mut app)
        .context("failed while running window event loop")
}

#[derive(Default)]
struct WindowApp {
    state: Option<GpuWindowState>,
}

impl ApplicationHandler for WindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("SnarkTerm")
            .with_inner_size(LogicalSize::new(1100.0, 720.0));

        match event_loop.create_window(attributes) {
            Ok(window) => match pollster::block_on(GpuWindowState::new(window)) {
                Ok(state) => self.state = Some(state),
                Err(error) => {
                    eprintln!("snarkterm: failed to initialize GPU renderer: {error:#}");
                    event_loop.exit();
                }
            },
            Err(error) => {
                eprintln!("snarkterm: failed to create window: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        if state.window.id() != id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => state.handle_key(&event),
            WindowEvent::ModifiersChanged(modifiers) => state.modifiers = modifiers.state(),
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.render() {
                    eprintln!("snarkterm: render failed: {error:#}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

struct GlyphInfo {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    width: f32,
    height: f32,
    x_offset: f32,
    y_offset: f32,
    #[allow(dead_code)]
    x_advance: f32,
}

struct GlyphCache {
    glyphs: Vec<GlyphInfo>,
    #[allow(dead_code)]
    atlas_width: f32,
    #[allow(dead_code)]
    atlas_height: f32,
    cell_w: f32,
    cell_h: f32,
    ascent: f32,
}

impl GlyphCache {
    fn new() -> Self {
        let font = FontRef::try_from_slice(EMBEDDED_FONT).expect("failed to load embedded font");
        let scale = PxScale::from(FONT_SIZE);
        let scaled = font.as_scaled(scale);

        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let line_gap = scaled.line_gap();
        let cell_h = ascent - descent + line_gap;
        let cell_w = scaled.h_advance(font.glyph_id('M'));

        let mut atlas_data = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];

        let mut glyphs = Vec::with_capacity(96);

        for i in 0u8..96 {
            let ch = (i + 32) as char;
            let glyph_id = font.glyph_id(ch);
            let glyph = glyph_id.with_scale_and_position(PxScale::from(FONT_SIZE), ab_glyph::point(0.0, 0.0));

            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                let glyph_w = bounds.width().max(1.0) as u32;
                let glyph_h = bounds.height().max(1.0) as u32;

                let atlas_col = i as u32 % ATLAS_COLS;
                let atlas_row = i as u32 / ATLAS_COLS;
                let atlas_x = atlas_col * ATLAS_CELL;
                let atlas_y = atlas_row * ATLAS_CELL;

                outlined.draw(|x, y, coverage| {
                    let px = (atlas_x + x as u32).min(ATLAS_SIZE - 1);
                    let py = (atlas_y + y as u32).min(ATLAS_SIZE - 1);
                    let idx = (py * ATLAS_SIZE + px) as usize;
                    if idx < atlas_data.len() {
                        let existing = atlas_data[idx];
                        let new_val = (coverage * 255.0) as u8;
                        atlas_data[idx] = existing.max(new_val);
                    }
                });

                let u0 = atlas_x as f32 / ATLAS_SIZE as f32;
                let v0 = atlas_y as f32 / ATLAS_SIZE as f32;
                let u1 = (atlas_x + glyph_w) as f32 / ATLAS_SIZE as f32;
                let v1 = (atlas_y + glyph_h) as f32 / ATLAS_SIZE as f32;

                glyphs.push(GlyphInfo {
                    u0,
                    v0,
                    u1,
                    v1,
                    width: glyph_w as f32,
                    height: glyph_h as f32,
                    x_offset: bounds.min.x as f32,
                    y_offset: bounds.min.y as f32,
                    x_advance: scaled.h_advance(glyph_id),
                });
            } else {
                let atlas_col = i as u32 % ATLAS_COLS;
                let atlas_row = i as u32 / ATLAS_COLS;
                let atlas_x = atlas_col * ATLAS_CELL;
                let atlas_y = atlas_row * ATLAS_CELL;

                let u0 = atlas_x as f32 / ATLAS_SIZE as f32;
                let v0 = atlas_y as f32 / ATLAS_SIZE as f32;

                glyphs.push(GlyphInfo {
                    u0,
                    v0,
                    u1: u0,
                    v1: v0,
                    width: 0.0,
                    height: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                    x_advance: scaled.h_advance(glyph_id),
                });
            }
        }

        Self {
            glyphs,
            atlas_width: ATLAS_SIZE as f32,
            atlas_height: ATLAS_SIZE as f32,
            cell_w,
            cell_h,
            ascent,
        }
    }

    fn glyph(&self, ch: char) -> &GlyphInfo {
        let idx = (ch as u32).saturating_sub(32).min(95) as usize;
        &self.glyphs[idx]
    }
}

struct GpuWindowState {
    window: &'static Window,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    bind_group: wgpu::BindGroup,
    terminal: Arc<Mutex<TerminalBuffer>>,
    pty_writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pty_master: Option<Box<dyn portable_pty::MasterPty>>,
    modifiers: ModifiersState,
    glyph_cache: GlyphCache,
}

impl GpuWindowState {
    async fn new(window: Window) -> Result<Self> {
        let window = Box::leak(Box::new(window));
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(&*window)
            .context("failed to create GPU surface")?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("no compatible GPU adapter found"))?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("snarkterm-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .context("failed to create GPU device")?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| anyhow!("surface reported no supported formats"))?;
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(wgpu::PresentMode::AutoVsync);
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let glyph_cache = GlyphCache::new();

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("snarkterm-glyph-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let mut atlas_data = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];
        let font =
            FontRef::try_from_slice(EMBEDDED_FONT).expect("failed to load embedded font for atlas");
        let scale = PxScale::from(FONT_SIZE);
        let scaled = font.as_scaled(scale);

        for i in 0u8..96u8 {
            let ch = (i + 32) as char;
            let glyph_id = font.glyph_id(ch);
            let glyph = glyph_id.with_scale_and_position(PxScale::from(FONT_SIZE), ab_glyph::point(0.0, 0.0));

            if let Some(outlined) = font.outline_glyph(glyph) {
                let _bounds = outlined.px_bounds();
                let atlas_col = i as u32 % ATLAS_COLS;
                let atlas_row = i as u32 / ATLAS_COLS;
                let atlas_x = atlas_col * ATLAS_CELL;
                let atlas_y = atlas_row * ATLAS_CELL;

                let _ = scaled;
                outlined.draw(|x, y, coverage| {
                    let px = (atlas_x + x as u32).min(ATLAS_SIZE - 1);
                    let py = (atlas_y + y as u32).min(ATLAS_SIZE - 1);
                    let idx = (py * ATLAS_SIZE + px) as usize;
                    if idx < atlas_data.len() {
                        let existing = atlas_data[idx];
                        let new_val = (coverage * 255.0) as u8;
                        atlas_data[idx] = existing.max(new_val);
                    }
                });
            }
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE),
                rows_per_image: Some(ATLAS_SIZE),
            },
            wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
        );

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("snarkterm-glyph-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("snarkterm-glyph-bind-group-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("snarkterm-glyph-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline = create_render_pipeline(&device, config.format, &bind_group_layout);
        let (cols, rows) = grid_size(config.width, config.height, &glyph_cache);
        let terminal = Arc::new(Mutex::new(TerminalBuffer::new(cols, rows)));
        let (pty_writer, pty_master) = spawn_window_shell(Arc::clone(&terminal))?;

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group,
            terminal,
            pty_writer,
            pty_master: Some(pty_master),
            modifiers: ModifiersState::empty(),
            glyph_cache,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let (cols, rows) = grid_size(width, height, &self.glyph_cache);
        if let Ok(mut terminal) = self.terminal.lock() {
            terminal.resize(cols, rows);
        }
        if let Some(ref master) = self.pty_master {
            let _ = master.resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    fn handle_key(&mut self, key: &winit::event::KeyEvent) {
        if key.state != ElementState::Pressed {
            return;
        }

        let ctrl = self.modifiers.control_key();
        let alt = self.modifiers.alt_key();

        let bytes: Option<&[u8]> = match &key.logical_key {
            Key::Named(NamedKey::Enter) => Some(b"\r"),
            Key::Named(NamedKey::Tab) => Some(b"\t"),
            Key::Named(NamedKey::Backspace) => Some(b"\x7f"),
            Key::Named(NamedKey::Escape) => Some(b"\x1b"),
            Key::Named(NamedKey::ArrowUp) => Some(b"\x1b[A"),
            Key::Named(NamedKey::ArrowDown) => Some(b"\x1b[B"),
            Key::Named(NamedKey::ArrowRight) => Some(b"\x1b[C"),
            Key::Named(NamedKey::ArrowLeft) => Some(b"\x1b[D"),
            Key::Named(NamedKey::Home) => Some(b"\x1b[H"),
            Key::Named(NamedKey::End) => Some(b"\x1b[F"),
            Key::Named(NamedKey::Delete) => Some(b"\x1b[3~"),
            Key::Named(NamedKey::PageUp) => {
                if let Ok(mut terminal) = self.terminal.lock() {
                    terminal.scroll_page_up();
                }
                None
            }
            Key::Named(NamedKey::PageDown) => {
                if let Ok(mut terminal) = self.terminal.lock() {
                    terminal.scroll_page_down();
                }
                None
            }
            Key::Character(ch) if ctrl => match ch.as_str() {
                "c" => Some(b"\x03"),
                "d" => Some(b"\x04"),
                "z" => Some(b"\x1a"),
                "l" => Some(b"\x0c"),
                "a" => Some(b"\x01"),
                "e" => Some(b"\x05"),
                "k" => Some(b"\x0b"),
                "u" => Some(b"\x15"),
                "w" => Some(b"\x17"),
                "r" => Some(b"\x12"),
                "s" => Some(b"\x13"),
                "g" => Some(b"\x07"),
                _ => {
                    if let Some(byte) = ch.as_bytes().first() {
                        if byte.is_ascii_lowercase() {
                            let ctrl_byte = byte - b'a' + 1;
                            Some(&[ctrl_byte])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            },
            Key::Character(ch) if alt => match ch.as_str() {
                "b" => Some(b"\x1bb"),
                "f" => Some(b"\x1bf"),
                "d" => Some(b"\x1bd"),
                "<" => Some(b"\x1b<"),
                _ => None,
            },
            _ => key.text.as_ref().map(|text| text.as_bytes()),
        };

        if let Some(bytes) = bytes {
            if let Ok(mut writer) = self.pty_writer.lock() {
                let _ = writer.write_all(bytes);
                let _ = writer.flush();
            }
        }
    }

    fn render(&mut self) -> Result<()> {
        let vertices = self.text_vertices();
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(error) => return Err(anyhow!(error)),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("snarkterm-text-vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("snarkterm-render-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("snarkterm-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.031,
                            g: 0.039,
                            b: 0.059,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if !vertices.is_empty() {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn text_vertices(&self) -> Vec<Vertex> {
        let Ok(terminal) = self.terminal.lock() else {
            return Vec::new();
        };

        let mut vertices = Vec::new();
        let screen_w = self.config.width as f32;
        let screen_h = self.config.height as f32;
        let gc = &self.glyph_cache;

        let white_u = (ATLAS_COLS * ATLAS_CELL - 1) as f32 / ATLAS_SIZE as f32;
        let white_v = (ATLAS_COLS * ATLAS_CELL - 1) as f32 / ATLAS_SIZE as f32;

        for (row, line) in terminal.display_cells().iter().enumerate() {
            for (col, cell) in line.iter().enumerate() {
                let x = GRID_MARGIN_X + col as f32 * gc.cell_w;
                let y = GRID_MARGIN_Y + row as f32 * gc.cell_h;

                if let Some(bg) = cell.bg {
                    push_quad(
                        &mut vertices,
                        x,
                        y,
                        gc.cell_w,
                        gc.cell_h,
                        screen_w,
                        screen_h,
                        bg.as_array(),
                        white_u,
                        white_v,
                        white_u,
                        white_v,
                    );
                }

                if cell.ch != ' ' && cell.ch != '\0' {
                    let g = gc.glyph(cell.ch);
                    if g.width > 0.0 {
                        let gx = x + g.x_offset;
                        let gy = y + gc.ascent + g.y_offset;
                        push_quad(
                            &mut vertices,
                            gx,
                            gy,
                            g.width,
                            g.height,
                            screen_w,
                            screen_h,
                            cell.fg.as_array(),
                            g.u0,
                            g.v0,
                            g.u1,
                            g.v1,
                        );
                    }
                }

                if cell.underline {
                    push_quad(
                        &mut vertices,
                        x,
                        y + gc.cell_h - 2.0,
                        gc.cell_w,
                        1.0,
                        screen_w,
                        screen_h,
                        cell.fg.as_array(),
                        white_u,
                        white_v,
                        white_u,
                        white_v,
                    );
                }

                if cell.strikethrough {
                    push_quad(
                        &mut vertices,
                        x,
                        y + gc.cell_h * 0.5,
                        gc.cell_w,
                        1.0,
                        screen_w,
                        screen_h,
                        cell.fg.as_array(),
                        white_u,
                        white_v,
                        white_u,
                        white_v,
                    );
                }
            }
        }

        let cursor_x = GRID_MARGIN_X + terminal.cursor_col as f32 * gc.cell_w;
        let cursor_y = GRID_MARGIN_Y + terminal.cursor_row as f32 * gc.cell_h + gc.cell_h - 2.0;
        push_quad(
            &mut vertices,
            cursor_x,
            cursor_y,
            gc.cell_w,
            2.0,
            screen_w,
            screen_h,
            [1.0, 0.23, 0.42],
            white_u,
            white_v,
            white_u,
            white_v,
        );

        vertices
    }
}

fn grid_size(width: u32, height: u32, gc: &GlyphCache) -> (usize, usize) {
    let cols =
        ((width as f32 - GRID_MARGIN_X * 2.0) / gc.cell_w).floor().max(1.0) as usize;
    let rows =
        ((height as f32 - GRID_MARGIN_Y * 2.0) / gc.cell_h).floor().max(1.0) as usize;
    (cols, rows)
}

fn spawn_window_shell(
    terminal: Arc<Mutex<TerminalBuffer>>,
) -> Result<(
    Arc<Mutex<Box<dyn Write + Send>>>,
    Box<dyn portable_pty::MasterPty>,
)> {
    let shell = user_shell();
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 36,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("failed to open window PTY")?;
    let mut command = CommandBuilder::new(&shell);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("SNARKTERM", "1");
    let mut child = pair
        .slave
        .spawn_command(command)
        .with_context(|| format!("failed to spawn shell '{shell}'"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .context("failed to clone window PTY reader")?;
    let writer = Arc::new(Mutex::new(
        pair.master
            .take_writer()
            .context("failed to take window PTY writer")?,
    ));

    thread::spawn(move || {
        let mut parser = OutputParser::default();
        let mut buffer = [0; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if let Ok(mut terminal) = terminal.lock() {
                        parser.feed(&buffer[..read], &mut terminal);
                    }
                }
                Err(_) => break,
            }
        }
        let _ = child.wait();
    });

    Ok((writer, pair.master))
}

fn create_render_pipeline(
    device: &Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("snarkterm-render-shader"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@group(0) @binding(0) var glyph_texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let a = textureSample(glyph_texture, tex_sampler, in.uv).r;
    return vec4<f32>(in.color, a);
}
"#
            .into(),
        ),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("snarkterm-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("snarkterm-render-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 20,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

fn push_quad(
    vertices: &mut Vec<Vertex>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    screen_w: f32,
    screen_h: f32,
    color: [f32; 3],
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
) {
    let x1 = x / screen_w * 2.0 - 1.0;
    let y1 = 1.0 - y / screen_h * 2.0;
    let x2 = (x + w) / screen_w * 2.0 - 1.0;
    let y2 = 1.0 - (y + h) / screen_h * 2.0;
    vertices.extend_from_slice(&[
        Vertex { position: [x1, y1], color, uv: [u0, v0] },
        Vertex { position: [x2, y1], color, uv: [u1, v0] },
        Vertex { position: [x2, y2], color, uv: [u1, v1] },
        Vertex { position: [x1, y1], color, uv: [u0, v0] },
        Vertex { position: [x2, y2], color, uv: [u1, v1] },
        Vertex { position: [x1, y2], color, uv: [u0, v1] },
    ]);
}

fn run_interactive() -> Result<u8> {
    let shell = user_shell();
    let size = terminal_size();
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(size).context("failed to open PTY")?;
    let _raw_mode = RawModeGuard::enable().context("failed to enable terminal raw mode")?;

    let mut command = CommandBuilder::new(&shell);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("SNARKTERM", "1");

    let mut child = pair
        .slave
        .spawn_command(command)
        .with_context(|| format!("failed to spawn shell '{shell}'"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .context("failed to clone PTY reader")?;
    let mut writer = pair
        .master
        .take_writer()
        .context("failed to take PTY writer")?;

    let output_thread = thread::spawn(move || -> io::Result<()> {
        let mut stdout = io::stdout();
        io::copy(&mut reader, &mut stdout)?;
        stdout.flush()
    });

    let _input_thread = thread::spawn(move || -> io::Result<()> {
        let mut stdin = io::stdin();
        io::copy(&mut stdin, &mut writer)?;
        writer.flush()
    });

    let status = child.wait().context("failed to wait for shell")?;
    let _ = output_thread.join();

    Ok(status.exit_code().min(u8::MAX as u32) as u8)
}

fn run_command(command: &str, no_commentary: bool) -> Result<u8> {
    let shell = user_shell();
    let size = terminal_size();
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(size).context("failed to open PTY")?;

    let mut shell_command = CommandBuilder::new(&shell);
    shell_command.args(["-lc", command]);
    shell_command.env("TERM", "xterm-256color");
    shell_command.env("COLORTERM", "truecolor");
    shell_command.env("SNARKTERM", "1");

    let mut child = pair
        .slave
        .spawn_command(shell_command)
        .with_context(|| format!("failed to run command with shell '{shell}'"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .context("failed to clone PTY reader")?;
    let output_thread = thread::spawn(move || -> io::Result<()> {
        let mut stdout = io::stdout();
        io::copy(&mut reader, &mut stdout)?;
        stdout.flush()
    });

    let status = child.wait().context("failed to wait for command")?;
    let _ = output_thread.join();
    let exit_code = status.exit_code().min(u8::MAX as u32) as u8;

    if !no_commentary {
        eprintln!("{}", command_commentary(command, exit_code));
    }

    Ok(exit_code)
}

fn command_commentary(command: &str, exit_code: u8) -> &'static str {
    if command.contains("rm -rf /") {
        "SnarkTerm: I see we've arrived at the burn down the library to find the bookmark phase."
    } else if command.contains("git push --force") || command.contains("git push -f") {
        "SnarkTerm: Force push detected. Somewhere, a future coworker just developed a migraine."
    } else if command.contains("chmod 777") {
        "SnarkTerm: Security model replaced with vibes. Very modern."
    } else if command.contains("curl") && (command.contains("| sh") || command.contains("| bash")) {
        "SnarkTerm: A stranger's shell script, piped directly into trust. Inspirationally backwards."
    } else if exit_code == 127 {
        "SnarkTerm: Command not found. Either it doesn't exist, or it's hiding from you."
    } else if exit_code == 126 {
        "SnarkTerm: Permission denied. The file exists but wants nothing to do with you."
    } else if exit_code == 130 {
        "SnarkTerm: Interrupted. YouCtrl+C'd your way out of that one."
    } else if exit_code == 137 {
        "SnarkTerm: Killed. Something decided this process had lived long enough."
    } else if exit_code == 0 {
        "SnarkTerm: Exit code 0. A rare and beautiful creature, like a printer that works."
    } else {
        "SnarkTerm: The command failed. A bold result, though not traditionally useful."
    }
}

fn user_shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

fn terminal_size() -> PtySize {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn print_help() {
    println!(
        "SnarkTerm {VERSION}\n\
\n\
Usage:\n\
  snarkterm [OPTIONS]\n\
  snarkterm -c <COMMAND>\n\
\n\
Options:\n\
  -c, --command <COMMAND>  Run a command through the user's shell and exit\n\
      --window            Launch the native winit/wgpu window preview\n\
      --no-commentary     Disable command-mode commentary\n\
  -h, --help              Show this help text\n\
  -V, --version           Show version\n\
\n\
Current status:\n\
  GPU-rendered terminal with proper font loading, PTY integration, scrollback,\n\
  color support, and Ctrl key bindings. Tabs, splits, and the snark gutter\n\
  are still in progress, because building a real terminal takes more than\n\
  one weekend and a sarcastic personality."
    );
}

struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::{command_commentary, parse_args, GlyphCache};

    #[test]
    fn parses_command_flag() {
        let args = parse_args(["--command".to_string(), "true".to_string()].into_iter()).unwrap();

        assert_eq!(args.command.as_deref(), Some("true"));
    }

    #[test]
    fn parses_window_flag() {
        let args = parse_args(["--window".to_string()].into_iter()).unwrap();

        assert!(args.window);
    }

    #[test]
    fn comments_on_failed_command() {
        let comment = command_commentary("false", 1);

        assert!(comment.contains("failed"));
    }

    #[test]
    fn comments_on_force_push() {
        let comment = command_commentary("git push --force", 0);

        assert!(comment.contains("future coworker"));
    }

    #[test]
    fn comments_on_command_not_found() {
        let comment = command_commentary("asdfghjkl", 127);
        assert!(comment.contains("not found"));
    }

    #[test]
    fn comments_on_permission_denied() {
        let comment = command_commentary("./myscript.sh", 126);
        assert!(comment.contains("Permission denied"));
    }

    #[test]
    fn comments_on_sigint() {
        let comment = command_commentary("sleep 999", 130);
        assert!(comment.contains("Interrupted"));
    }

    #[test]
    fn comments_on_sigkill() {
        let comment = command_commentary("heavy_process", 137);
        assert!(comment.contains("Killed"));
    }

    #[test]
    fn comments_on_success() {
        let comment = command_commentary("echo hi", 0);
        assert!(comment.contains("Exit code 0"));
    }

    #[test]
    fn comments_on_generic_failure() {
        let comment = command_commentary("make", 2);
        assert!(comment.contains("failed"));
    }

    #[test]
    fn glyph_cache_loads_all_printable_ascii() {
        let gc = GlyphCache::new();
        assert_eq!(gc.glyphs.len(), 96);
        assert!(gc.cell_w > 0.0);
        assert!(gc.cell_h > 0.0);
    }

    #[test]
    fn glyph_cache_returns_valid_info_for_common_chars() {
        let gc = GlyphCache::new();
        let g = gc.glyph('A');
        assert!(g.width > 0.0);
        assert!(g.height > 0.0);
        assert!(g.x_advance > 0.0);

        let g = gc.glyph(' ');
        assert!(g.x_advance > 0.0);
    }

    #[test]
    fn grid_size_uses_font_metrics() {
        let gc = GlyphCache::new();
        let (cols, rows) = super::grid_size(1100, 720, &gc);
        assert!(cols > 40);
        assert!(rows > 15);
    }
}
