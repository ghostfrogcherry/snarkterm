use anyhow::{anyhow, Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use std::thread;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Default)]
struct Args {
    command: Option<String>,
    no_commentary: bool,
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

    run_interactive()
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args> {
    let mut parsed = Args::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => parsed.help = true,
            "-V" | "--version" => parsed.version = true,
            "--no-commentary" => parsed.no_commentary = true,
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
      --no-commentary     Disable command-mode commentary\n\
  -h, --help              Show this help text\n\
  -V, --version           Show version\n\
\n\
Current status:\n\
  This is the first usable PTY-backed cut. It launches a real shell and passes\n\
  bytes like a terminal. The GPU window, tabs, splits, and side gutter are still\n\
  upcoming, because apparently serious software requires implementation."
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
