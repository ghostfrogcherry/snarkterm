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
