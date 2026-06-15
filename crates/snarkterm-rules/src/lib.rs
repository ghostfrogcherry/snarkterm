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
