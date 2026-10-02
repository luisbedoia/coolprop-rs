use std::ffi::NulError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsError {
    /// The name matches no curated fluid nor any of its aliases.
    UnknownFluid(String),
    /// The request itself is malformed (e.g. an unsupported input pair).
    InvalidInput(String),
    /// CoolProp rejected or failed to solve the request (e.g. a state
    /// outside the EOS range).
    CoolProp(String),
}

impl fmt::Display for PropsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFluid(name) => write!(f, "unknown fluid: {name}"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::CoolProp(msg) => write!(f, "CoolProp error: {msg}"),
        }
    }
}

impl std::error::Error for PropsError {}

impl From<NulError> for PropsError {
    fn from(err: NulError) -> Self {
        Self::InvalidInput(err.to_string())
    }
}
