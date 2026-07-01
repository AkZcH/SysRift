use thiserror::Error;

#[derive(Debug, Error)]
pub enum SysriftError {
    #[error("fork failed: {0}")]
    Fork(#[from] nix::Error),

    #[error("ptrace error: {0}")]
    Ptrace(nix::Error),

    #[error("trace log error: {0}")]
    Io(#[from] std::io::Error),

    #[error("trace log parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("target program not found: {0}")]
    ExecFailed(nix::Error),
}

pub type Result<T> = std::result::Result<T, SysriftError>;
