use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured error categories returned by Tenon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    InvalidInput,
    NotFound,
    Unsupported,
    Conflict,
    Busy,
    PlanExpired,
    Protected,
    Authorization,
    CommandFailed,
    Persistence,
    Internal,
}

/// Library error combining a machine-readable category with a user-facing message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unsupported, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    pub fn busy(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Busy, message)
    }

    pub fn plan_expired(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::PlanExpired, message)
    }

    pub fn protected(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Protected, message)
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Authorization, message)
    }

    pub fn command_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::CommandFailed, message)
    }

    pub fn persistence(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Persistence, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}
