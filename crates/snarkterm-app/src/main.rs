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
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

struct GpuWindowState {
    window: &'static Window,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    terminal: Arc<Mutex<TerminalBuffer>>,
    pty_writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl GpuWindowState {
    async fn new(window: Window) -> Result<Self> {
        let window = Box::leak(Box::new(window));
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(&*window).context("failed to create GPU surface")?;
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
        let pipeline = create_text_pipeline(&device, config.format);
        let terminal = Arc::new(Mutex::new(TerminalBuffer::new(80, 36)));
        let pty_writer = spawn_window_shell(Arc::clone(&terminal))?;

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            terminal,
            pty_writer,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn handle_key(&mut self, key: &winit::event::KeyEvent) {
        if key.state != ElementState::Pressed {
            return;
        }

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
                label: Some("snarkterm-background-pass"),
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
        let margin_x = 16.0;
        let margin_y = 18.0;
        let cell_w = 10.0;
        let cell_h = 16.0;
        let pixel = 2.0;
        let width = self.config.width as f32;
        let height = self.config.height as f32;

        for (row, line) in terminal.cells().iter().enumerate() {
            for (col, cell) in line.iter().enumerate() {
                if cell.ch == ' ' {
                    continue;
                }
                let x = margin_x + col as f32 * cell_w;
                let y = margin_y + row as f32 * cell_h;
                push_glyph(
                    &mut vertices,
                    cell.ch,
                    x,
                    y,
                    pixel,
                    width,
                    height,
                    cell.fg.as_array(),
                );
            }
        }

        let cursor_x = margin_x + terminal.cursor_col as f32 * cell_w;
        let cursor_y = margin_y + terminal.cursor_row as f32 * cell_h + 14.0;
        push_rect(
            &mut vertices,
            cursor_x,
            cursor_y,
            8.0,
            2.0,
            width,
            height,
            [1.0, 0.23, 0.42],
        );

        vertices
    }
}

fn spawn_window_shell(terminal: Arc<Mutex<TerminalBuffer>>) -> Result<Arc<Mutex<Box<dyn Write + Send>>>> {
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

    Ok(writer)
}

fn create_text_pipeline(device: &Device, format: wgpu::TextureFormat) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("snarkterm-bitmap-text-shader"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) color: vec3<f32>) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#
            .into(),
        ),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("snarkterm-text-pipeline-layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("snarkterm-text-pipeline"),
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

fn push_glyph(
    vertices: &mut Vec<Vertex>,
    ch: char,
    x: f32,
    y: f32,
    pixel: f32,
    width: f32,
    height: f32,
    color: [f32; 3],
) {
    for (row, bits) in glyph_rows(ch).iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                push_rect(
                    vertices,
                    x + col as f32 * pixel,
                    y + row as f32 * pixel,
                    pixel,
                    pixel,
                    width,
                    height,
                    color,
                );
            }
        }
    }
}

fn push_rect(
    vertices: &mut Vec<Vertex>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    screen_w: f32,
    screen_h: f32,
    color: [f32; 3],
) {
    let x1 = x / screen_w * 2.0 - 1.0;
    let y1 = 1.0 - y / screen_h * 2.0;
    let x2 = (x + w) / screen_w * 2.0 - 1.0;
    let y2 = 1.0 - (y + h) / screen_h * 2.0;
    vertices.extend_from_slice(&[
        Vertex { position: [x1, y1], color },
        Vertex { position: [x2, y1], color },
        Vertex { position: [x2, y2], color },
        Vertex { position: [x1, y1], color },
        Vertex { position: [x2, y2], color },
        Vertex { position: [x1, y2], color },
    ]);
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
        '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000],
        ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
        ';' => [0b00000, 0b01100, 0b01100, 0b00000, 0b00100, 0b00100, 0b01000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '\\' => [0b10000, 0b01000, 0b01000, 0b00100, 0b00010, 0b00010, 0b00001],
        '|' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        '>' => [0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000],
        '<' => [0b00001, 0b00010, 0b00100, 0b01000, 0b00100, 0b00010, 0b00001],
        '=' => [0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '*' => [0b00000, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00000],
        '$' => [0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        '{' => [0b00010, 0b00100, 0b00100, 0b01000, 0b00100, 0b00100, 0b00010],
        '}' => [0b01000, 0b00100, 0b00100, 0b00010, 0b00100, 0b00100, 0b01000],
        '~' => [0b00000, 0b00000, 0b01000, 0b10101, 0b00010, 0b00000, 0b00000],
        '@' => [0b01110, 0b10001, 0b10111, 0b10101, 0b10111, 0b10000, 0b01110],
        '&' => [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101],
        '%' => [0b11000, 0b11001, 0b00010, 0b00100, 0b01000, 0b10011, 0b00011],
        '^' => [0b00100, 0b01010, 0b10001, 0b00000, 0b00000, 0b00000, 0b00000],
        '"' => [0b01010, 0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000],
        '\'' => [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000],
        '`' => [0b01000, 0b00100, 0b00010, 0b00000, 0b00000, 0b00000, 0b00000],
        _ => [0b11111, 0b10001, 0b00101, 0b01001, 0b10100, 0b10001, 0b11111],
    }
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
  This is the first usable PTY-backed cut. It launches a real shell and passes\n\
  bytes like a terminal. The --window path opens the first native GPU surface.\n\
  Tabs, splits, text rendering, and the side gutter are still upcoming, because\n\
  apparently serious software requires implementation."
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
    use super::{command_commentary, parse_args};

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
    fn glyph_rows_supports_letters() {
        assert_ne!(super::glyph_rows('S'), [0; 7]);
    }
}
