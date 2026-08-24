use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NovaError {
    pub line: usize,
    pub col: Option<usize>,
    pub msg: String,
}

impl NovaError {
    pub fn new(line: usize, col: Option<usize>, msg: impl Into<String>) -> Self {
        NovaError { line, col, msg: msg.into() }
    }
}

impl fmt::Display for NovaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

impl std::error::Error for NovaError {}

pub type Result<T> = std::result::Result<T, NovaError>;
