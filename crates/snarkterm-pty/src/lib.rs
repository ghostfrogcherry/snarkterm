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

#[cfg(test)]
mod tests {
    use super::{parse_marker, ShellMarker};

    #[test]
    fn parses_prompt_start_marker() {
        let marker = parse_marker("snarkterm;event=prompt_start;cwd=/tmp");

        assert_eq!(marker, Some(ShellMarker::PromptStart));
    }

    #[test]
    fn ignores_non_snarkterm_marker() {
        let marker = parse_marker("not-snarkterm;event=prompt_start");

        assert_eq!(marker, None);
    }

    #[test]
    fn parses_command_end_marker() {
        let marker = parse_marker("snarkterm;event=command_end;status=1;duration_ms=842");

        assert_eq!(
            marker,
            Some(ShellMarker::CommandEnd {
                status: None,
                duration_ms: None,
            })
        );
    }
}
