//! Source locations and spans. Every AST/HIR/CGIR node carries one.
//! Critical for provenance (book insists: design in from M0/M3).

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct SourceId(pub u32);

#[allow(clippy::derivable_impls)]
impl Default for SourceId {
    fn default() -> Self {
        SourceId(0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub source: SourceId,
    pub start: u32,
    pub end: u32, // exclusive
}

impl Span {
    pub fn new(source: SourceId, start: u32, end: u32) -> Self {
        Span { source, start, end }
    }

    pub fn dummy() -> Self {
        Span {
            source: SourceId(0),
            start: 0,
            end: 0,
        }
    }

    #[inline]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub fn merge(self, other: Span) -> Span {
        if self.source != other.source {
            return self; // conservative
        }
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        Span {
            source: self.source,
            start,
            end,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Spanned { node, span }
    }
}
