use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

mod terminal_grid;

pub use terminal_grid::{Cell, OutputParser, RgbColor, TerminalBuffer};

pub type SessionId = Uuid;
pub type CommandId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub id: CommandId,
    pub session_id: SessionId,
    pub command: String,
    pub cwd: Option<String>,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub info: CommandInfo,
    pub exit_status: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DangerousCommand {
    pub command_id: Option<CommandId>,
    pub command: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongRunningInfo {
    pub command_id: CommandId,
    pub command: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistakeInfo {
    pub normalized_command: String,
    pub failure_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStats {
    pub failed_commands: u64,
    pub sudo_count: u64,
    pub force_push_count: u64,
    pub restart_fix_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalEvent {
    CommandStarted(CommandInfo),
    CommandCompleted(CommandResult),
    CommandFailed(CommandResult),
    DangerousCommandDetected(DangerousCommand),
    LongRunningCommand(LongRunningInfo),
    RepeatedMistake(MistakeInfo),
    SessionMilestone(SessionStats),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PersonalityProfile {
    Professional,
    Snarky,
    Unhinged,
    British,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CommentarySeverity {
    Info,
    Success,
    Warning,
    Danger,
    Achievement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commentary {
    pub text: String,
    pub severity: CommentarySeverity,
    pub personality: PersonalityProfile,
    pub created_at: DateTime<Utc>,
    pub ttl_ms: Option<u64>,
    pub related_command_id: Option<CommandId>,
}

impl Commentary {
    pub fn new(
        text: impl Into<String>,
        severity: CommentarySeverity,
        personality: PersonalityProfile,
    ) -> Self {
        Self {
            text: text.into(),
            severity,
            personality,
            created_at: Utc::now(),
            ttl_ms: Some(Duration::from_secs(12).as_millis() as u64),
            related_command_id: None,
        }
    }
}
