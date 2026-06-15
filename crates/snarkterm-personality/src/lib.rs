use snarkterm_core::{Commentary, CommentarySeverity, PersonalityProfile, TerminalEvent};

pub fn canned_commentary(event: &TerminalEvent, profile: PersonalityProfile, intensity: u8) -> Vec<Commentary> {
    let text = match (event, profile) {
        (TerminalEvent::CommandCompleted(_), PersonalityProfile::Professional) => {
            "Command completed successfully. Nobody was harmed, statistically speaking."
        }
        (TerminalEvent::CommandCompleted(_), PersonalityProfile::Snarky) => {
            "Exit code 0. A rare and beautiful creature, like a printer that works."
        }
        (TerminalEvent::CommandFailed(_), PersonalityProfile::British) => {
            "A bold command. Not a correct one, naturally, but bold."
        }
        (TerminalEvent::CommandFailed(_), PersonalityProfile::Unhinged) => {
            "The command failed again. At this point the bug has squatters' rights."
        }
        (TerminalEvent::DangerousCommandDetected(_), _) => {
            "I see we've arrived at the burn down the library to find the bookmark phase."
        }
        (TerminalEvent::LongRunningCommand(_), _) if intensity > 70 => {
            "Still running. Somewhere in there is a crate named after a woodland animal doing string parsing."
        }
        _ => "Noted. Filed gently under evidence.",
    };

    vec![Commentary::new(text, CommentarySeverity::Info, profile)]
}

#[cfg(test)]
mod tests {
    use super::canned_commentary;
    use chrono::Utc;
    use snarkterm_core::{CommandInfo, CommandResult, PersonalityProfile, TerminalEvent};
    use uuid::Uuid;

    fn completed_event() -> TerminalEvent {
        let info = CommandInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            command: "true".to_string(),
            cwd: Some("/tmp".to_string()),
            started_at: Utc::now(),
        };

        TerminalEvent::CommandCompleted(CommandResult {
            info,
            exit_status: Some(0),
            duration_ms: 10,
        })
    }

    #[test]
    fn snarky_success_mentions_printer() {
        let comments = canned_commentary(&completed_event(), PersonalityProfile::Snarky, 65);

        assert_eq!(comments.len(), 1);
        assert!(comments[0].text.contains("printer"));
    }

    #[test]
    fn professional_success_stays_restrained() {
        let comments = canned_commentary(&completed_event(), PersonalityProfile::Professional, 5);

        assert_eq!(comments.len(), 1);
        assert!(comments[0].text.contains("Command completed successfully"));
    }
}
