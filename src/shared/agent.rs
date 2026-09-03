use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Agent {
    Claude,
    Codex,
    Pi,
}

pub fn as_str(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude",
        Agent::Codex => "codex",
        Agent::Pi => "pi",
    }
}
