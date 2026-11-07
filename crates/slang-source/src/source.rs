use serde::Serialize;
use std::sync::Arc;

/// Index of an immutable source file within a compilation session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct SourceId(pub usize);

/// Half-open byte range. All compiler-produced boundaries are UTF-8 boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct Span {
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(source: SourceId, start: usize, end: usize) -> Self {
        Self { source, start, end }
    }

    pub fn join(self, other: Self) -> Self {
        if self.source == other.source {
            Self::new(
                self.source,
                self.start.min(other.start),
                self.end.max(other.end),
            )
        } else {
            self
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub id: SourceId,
    pub name: String,
    pub text: Arc<str>,
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// One-based line and Unicode scalar column, including the EOF position.
    pub fn location(&self, offset: usize) -> (usize, usize) {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        (
            line + 1,
            self.text[self.line_starts[line]..offset].chars().count() + 1,
        )
    }

    pub fn line(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len());
        Some(self.text[start..end].trim_end_matches(['\r', '\n']))
    }

    pub fn slice(&self, span: Span) -> Option<&str> {
        (span.source == self.id)
            .then(|| self.text.get(span.start..span.end))
            .flatten()
    }
}

#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn add(&mut self, name: impl Into<String>, text: impl Into<Arc<str>>) -> SourceId {
        let text = text.into();
        let id = SourceId(self.files.len());
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(offset, _)| offset + 1));
        self.files.push(SourceFile {
            id,
            name: name.into(),
            text,
            line_starts,
        });
        id
    }

    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.0)
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_crlf_and_eof_locations() {
        let mut map = SourceMap::default();
        let id = map.add("test", "α\r\nβ\n");
        let file = map.get(id).unwrap();
        assert_eq!(file.location(2), (1, 2));
        assert_eq!(file.location(4), (2, 1));
        assert_eq!(file.location(7), (3, 1));
        assert_eq!(file.line(1), Some("α"));
        assert_eq!(file.slice(Span::new(id, 0, 1)), None);
    }
}
