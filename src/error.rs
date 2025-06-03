use colored::*;
use std::fmt;

#[derive(Debug)]
pub struct CodeDebtError {
    pub message: String,
    pub suggestion: Option<String>,
}

impl CodeDebtError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl fmt::Display for CodeDebtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", "Error:".red().bold(), self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n{} {}", "Suggestion:".yellow().bold(), suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for CodeDebtError {}

pub fn handle_path_error(path: &str) -> CodeDebtError {
    if !std::path::Path::new(path).exists() {
        CodeDebtError::new(format!("Path '{}' does not exist", path))
            .with_suggestion("Check the path and try again. Use '.' for current directory.")
    } else if !std::path::Path::new(path).is_dir() {
        CodeDebtError::new(format!("Path '{}' is not a directory", path))
            .with_suggestion("Please provide a directory path, not a file.")
    } else {
        CodeDebtError::new(format!("Cannot access path '{}'", path))
            .with_suggestion("Check permissions and try again.")
    }
}

pub fn handle_git_error() -> CodeDebtError {
    CodeDebtError::new("Git repository not found")
        .with_suggestion("Run 'git init' first, or disable git features with --no-git-blame")
}
