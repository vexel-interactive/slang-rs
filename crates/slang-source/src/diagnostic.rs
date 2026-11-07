use crate::{SourceMap, Span};
use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &str, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            span,
            notes: vec![],
        }
    }

    pub fn warning(code: &str, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, span, message)
        }
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn render(&self, sources: &SourceMap) -> String {
        let mut result = self.to_string();
        if let Some(file) = sources.get(self.span.source) {
            let physical_line = file.location(self.span.start).0;
            let location = file.presumed_location(self.span.start);
            let (line, column) = (location.line, location.column);
            result.push_str(&format!("\n --> {}:{line}:{column}", location.name));
            if let Some(text) = file.line(physical_line) {
                result.push_str(&format!(
                    "\n {line} | {text}\n   | {}^",
                    " ".repeat(column - 1)
                ));
            }
        }
        for note in &self.notes {
            result.push_str(&format!("\n note: {note}"));
        }
        result
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{severity}[{}]: {}", self.code, self.message)
    }
}

impl std::error::Error for Diagnostic {}
