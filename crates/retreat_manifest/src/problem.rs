use std::fmt;

/// One thing wrong with a module (or with a selection of modules). `module` is the module id or
/// crate name it concerns; `message` says what is wrong and what to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub module: String,
    pub message: String,
}

impl Problem {
    pub fn new(module: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            module: module.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "module `{}`: {}", self.module, self.message)
    }
}
