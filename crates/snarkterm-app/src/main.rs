use anyhow::{anyhow, Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use std::thread;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
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

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
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

    fn render(&mut self) -> Result<()> {
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
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("snarkterm-render-encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
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
}
