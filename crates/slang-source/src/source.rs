use serde::Serialize;
use std::sync::Arc;

/// Index of a source file within a compilation session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct SourceId(pub usize);

/// Half-open byte range. All compiler-produced boundaries are UTF-8 boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct Span {
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
    /// Distinguishes instantiated syntax while retaining its original source location.
    #[serde(skip_serializing_if = "is_zero")]
    pub context: usize,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

impl Span {
    pub fn new(source: SourceId, start: usize, end: usize) -> Self {
        Self {
            source,
            start,
            end,
            context: 0,
        }
    }

    pub fn with_context(self, context: usize) -> Self {
        Self { context, ..self }
    }

    pub fn join(self, other: Self) -> Self {
        if self.source == other.source && self.context == other.context {
            Self::new(
                self.source,
                self.start.min(other.start),
                self.end.max(other.end),
            )
            .with_context(self.context)
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
    line_directives: Vec<LineMapping>,
}

/// Display coordinates after applying source line directives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation<'a> {
    pub name: &'a str,
    pub line: i32,
    pub column: usize,
}

/// A source line directive, independent of preprocessing token syntax.
#[derive(Clone, Debug)]
pub enum LineDirective {
    Default,
    Remap { line: i32, name: Option<String> },
}

#[derive(Clone, Debug)]
struct LineMapping {
    offset: usize,
    adjustment: i32,
    name: Option<String>,
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

    /// Source display coordinates; physical byte spans and file identity stay intact.
    pub fn presumed_location(&self, offset: usize) -> SourceLocation<'_> {
        let (line, column) = self.location(offset);
        let index = self
            .line_directives
            .partition_point(|d| d.offset <= offset.min(self.text.len()));
        let mapping = index.checked_sub(1).map(|i| &self.line_directives[i]);
        SourceLocation {
            name: mapping
                .and_then(|d| d.name.as_deref())
                .unwrap_or(&self.name),
            line: (line as i32).wrapping_add(mapping.map_or(0, |d| d.adjustment)),
            column,
        }
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
        line_starts.extend(text.bytes().enumerate().filter_map(|(offset, byte)| {
            (byte == b'\n' || byte == b'\r' && text.as_bytes().get(offset + 1) != Some(&b'\n'))
                .then_some(offset + 1)
        }));
        self.files.push(SourceFile {
            id,
            name: name.into(),
            text,
            line_starts,
            line_directives: vec![],
        });
        id
    }

    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.0)
    }

    /// Record a directive at its keyword offset, in strictly increasing source order.
    /// A remapped line number describes the physical line after the keyword's line.
    pub fn add_line_directive(
        &mut self,
        id: SourceId,
        offset: usize,
        directive: LineDirective,
    ) -> Result<(), &'static str> {
        let file = self.files.get_mut(id.0).ok_or("invalid source ID")?;
        if offset > file.text.len() || !file.text.is_char_boundary(offset) {
            return Err("line directive offset is outside a source character boundary");
        }
        if file
            .line_directives
            .last()
            .is_some_and(|d| d.offset >= offset)
        {
            return Err("line directives must be recorded in source order");
        }
        let mapping = match directive {
            LineDirective::Default => LineMapping {
                offset,
                adjustment: 0,
                name: None,
            },
            LineDirective::Remap { line, name } => {
                let name = name.unwrap_or_else(|| file.presumed_location(offset).name.to_owned());
                LineMapping {
                    offset,
                    adjustment: line
                        .wrapping_sub(file.location(offset).0 as i32)
                        .wrapping_sub(1),
                    name: Some(name),
                }
            }
        };
        file.line_directives.push(mapping);
        Ok(())
    }

    /// Clear mapping metadata before preprocessing an existing source again.
    pub fn clear_line_directives(&mut self, id: SourceId) -> Result<(), &'static str> {
        self.files
            .get_mut(id.0)
            .ok_or("invalid source ID")?
            .line_directives
            .clear();
        Ok(())
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

    #[test]
    fn physical_and_presumed_coordinates_keep_independent_names_and_lines() {
        let mut map = SourceMap::default();
        let text = "#line 50 \"virtual\"\r\nαβ\r#line 0\nγ\n#line default\nδ";
        let id = map.add("physical", text);
        let first = text.find("line").unwrap();
        let second = text.find("line 0").unwrap();
        let reset = text.find("line default").unwrap();
        map.add_line_directive(
            id,
            first,
            LineDirective::Remap {
                line: 50,
                name: Some("virtual".into()),
            },
        )
        .unwrap();
        map.add_line_directive(
            id,
            second,
            LineDirective::Remap {
                line: 0,
                name: None,
            },
        )
        .unwrap();
        map.add_line_directive(id, reset, LineDirective::Default)
            .unwrap();
        let file = map.get(id).unwrap();
        assert_eq!(file.presumed_location(0).name, "physical");
        assert_eq!(file.presumed_location(first).line, 49);
        let beta = text.find('β').unwrap();
        assert_eq!(file.location(beta), (2, 2));
        assert_eq!(
            file.presumed_location(beta),
            SourceLocation {
                name: "virtual",
                line: 50,
                column: 2
            }
        );
        assert_eq!(file.line(2), Some("αβ"));
        assert_eq!(file.line(3), Some("#line 0"));
        assert_eq!(file.presumed_location(text.find('γ').unwrap()).line, 0);
        assert_eq!(
            file.presumed_location(text.find('γ').unwrap()).name,
            "virtual"
        );
        assert_eq!(
            file.presumed_location(text.len()),
            SourceLocation {
                name: "physical",
                line: 6,
                column: 2
            }
        );
        map.clear_line_directives(id).unwrap();
        assert_eq!(
            map.get(id).unwrap().presumed_location(beta),
            SourceLocation {
                name: "physical",
                line: 2,
                column: 2
            }
        );
    }

    #[test]
    fn line_directives_validate_offsets_source_ids_and_order() {
        let mut map = SourceMap::default();
        let id = map.add("test", "α\nβ");
        assert!(
            map.add_line_directive(SourceId(1), 0, LineDirective::Default)
                .is_err()
        );
        assert!(map.clear_line_directives(SourceId(1)).is_err());
        for offset in [1, 6] {
            assert!(
                map.add_line_directive(id, offset, LineDirective::Default)
                    .is_err()
            );
        }
        map.add_line_directive(id, 2, LineDirective::Default)
            .unwrap();
        for offset in [0, 2] {
            assert!(
                map.add_line_directive(id, offset, LineDirective::Default)
                    .is_err()
            );
        }
    }

    #[test]
    fn presumed_signed_line_arithmetic_wraps_without_losing_physical_excerpts() {
        let mut map = SourceMap::default();
        let id = map.add("physical", "#line\nα\nβ");
        map.add_line_directive(
            id,
            1,
            LineDirective::Remap {
                line: i32::MIN,
                name: Some("virtual".into()),
            },
        )
        .unwrap();
        let file = map.get(id).unwrap();
        assert_eq!(file.presumed_location(1).line, i32::MAX);
        assert_eq!(file.presumed_location(6).line, i32::MIN);
        assert_eq!(file.presumed_location(file.text.len()).line, i32::MIN + 1);
        let rendered =
            crate::Diagnostic::error("TEST", Span::new(id, 6, 8), "failure").render(&map);
        assert!(rendered.contains("virtual:-2147483648:1"), "{rendered}");
        assert!(rendered.contains("-2147483648 | α"), "{rendered}");
    }
}
