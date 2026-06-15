pub const SNARKTERM_OSC: u16 = 777;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellMarker {
    PromptStart,
    CommandStart { command: String, cwd: Option<String> },
    CommandEnd { status: Option<i32>, duration_ms: Option<u64> },
}

pub fn parse_marker(payload: &str) -> Option<ShellMarker> {
    if !payload.starts_with("snarkterm;") {
        return None;
    }

    let fields = payload.trim_start_matches("snarkterm;");
    if fields.contains("event=prompt_start") {
        return Some(ShellMarker::PromptStart);
    }
    if fields.contains("event=command_start") {
        return Some(ShellMarker::CommandStart { command: String::new(), cwd: None });
    }
    if fields.contains("event=command_end") {
        return Some(ShellMarker::CommandEnd { status: None, duration_ms: None });
    }
    None
}
