//! The evaluated document behind the editor: text, one [`LineOutput`] per line, and the
//! highlighter that feeds the engine's spans to the gpui-base editor.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use gpui_kit::base::input::{
    EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, Rope,
};
use gpui_kit::{Context, HighlightStyle, SharedString, Window};
use napkin_core::{Answer, Engine, LineOutput, LineResult, SpanKind};

/// The engine shared by every evaluator in the app (editor, HTTP endpoint).
pub(crate) type SharedEngine = Arc<RwLock<Engine>>;

/// Language name the editor reports for Napkin documents.
pub(crate) const LANGUAGE: &str = "napkin";

/// A document and its evaluation, kept in sync with the editor text.
#[derive(Debug)]
pub(crate) struct Sheet {
    engine: SharedEngine,
    text: String,
    lines: Vec<LineOutput>,
    /// Byte offset of each line's first character in `text`.
    line_starts: Vec<usize>,
}

impl Sheet {
    pub(crate) fn new(engine: SharedEngine) -> Self {
        let mut sheet = Self {
            engine,
            text: String::new(),
            lines: Vec::new(),
            line_starts: Vec::new(),
        };
        sheet.reevaluate();
        sheet
    }

    /// Replaces the text and re-evaluates it; returns whether the text changed.
    pub(crate) fn set_text(&mut self, text: &str) -> bool {
        if self.text == text && !self.lines.is_empty() {
            return false;
        }
        text.clone_into(&mut self.text);
        self.reevaluate();
        true
    }

    /// Evaluates the current text again (after a settings, rates or extension change).
    pub(crate) fn reevaluate(&mut self) {
        let lines = self
            .engine
            .read()
            .expect("engine lock poisoned")
            .evaluate(&self.text);
        self.lines = lines;
        self.line_starts = line_starts(&self.text);
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// The computed value of `line`, if it has one (errors show nothing, like Numi).
    pub(crate) fn answer(&self, line: usize) -> Option<&Answer> {
        match &self.lines.get(line)?.result {
            LineResult::Value(answer) => Some(answer),
            LineResult::None | LineResult::Error(_) => None,
        }
    }

    /// Every line's answer text, `None` for lines without a value.
    pub(crate) fn answers(&self) -> impl Iterator<Item = Option<&Answer>> {
        (0..self.lines.len()).map(|line| self.answer(line))
    }

    /// The source text of `line`, without its newline.
    pub(crate) fn line_text(&self, line: usize) -> &str {
        let Some(&start) = self.line_starts.get(line) else {
            return "";
        };
        let end = self
            .line_starts
            .get(line + 1)
            .map_or(self.text.len(), |next| next - 1);
        &self.text[start..end]
    }

    /// The line containing byte `offset`.
    pub(crate) fn line_at(&self, offset: usize) -> usize {
        self.line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1)
    }

    /// Byte range of `line` in the document, without its newline.
    pub(crate) fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.line_starts.get(line).copied().unwrap_or(self.text.len());
        start..start + self.line_text(line).len()
    }

    /// Highlight runs that cover `range` completely, in order: engine spans with their kind,
    /// everything between them with `None`.
    pub(crate) fn runs(&self, range: &Range<usize>) -> Vec<(Range<usize>, Option<SpanKind>)> {
        let end = range.end.min(self.text.len());
        let mut runs = Vec::new();
        let mut cursor = range.start.min(end);
        let mut line = self.line_at(cursor);
        while cursor < end && line < self.line_starts.len() {
            let line_start = self.line_starts[line];
            let line_text = self.line_text(line);
            if let Some(output) = self.lines.get(line) {
                for span in &output.spans {
                    let span_start = (line_start + span.range.start.min(line_text.len())).max(cursor);
                    let span_end = (line_start + span.range.end.min(line_text.len())).min(end);
                    let on_char_boundaries =
                        self.text.is_char_boundary(span_start) && self.text.is_char_boundary(span_end);
                    if span_start >= span_end || !on_char_boundaries {
                        continue;
                    }
                    if cursor < span_start {
                        runs.push((cursor..span_start, None));
                    }
                    runs.push((span_start..span_end, Some(span.kind)));
                    cursor = span_end;
                }
            }
            line += 1;
        }
        if cursor < end {
            runs.push((cursor..end, None));
        }
        // The editor may ask for bytes past our text while an edit is in flight.
        if range.end > end {
            runs.push((end.max(range.start)..range.end, None));
        }
        runs
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(index, _)| index + 1))
        .collect()
}

/// The highlight name a span kind resolves through [`HighlightStyleResolver`].
pub(crate) fn kind_name(kind: SpanKind) -> &'static str {
    match kind {
        SpanKind::Number => "napkin.number",
        SpanKind::Unit => "napkin.unit",
        SpanKind::Currency => "napkin.currency",
        SpanKind::Operator => "napkin.operator",
        SpanKind::Keyword => "napkin.keyword",
        SpanKind::Variable => "napkin.variable",
        SpanKind::Function => "napkin.function",
        SpanKind::Constant => "napkin.constant",
        SpanKind::DateTime => "napkin.datetime",
        SpanKind::Comment => "napkin.comment",
        SpanKind::Header => "napkin.header",
        SpanKind::Label => "napkin.label",
        SpanKind::Text => "napkin.text",
    }
}

/// Feeds the editor's text into the [`Sheet`] and its spans back as highlight styles.
pub(crate) struct SheetHighlighter {
    sheet: Rc<RefCell<Sheet>>,
}

impl SheetHighlighter {
    pub(crate) fn new(sheet: Rc<RefCell<Sheet>>) -> Self {
        Self { sheet }
    }
}

impl InputHighlighter for SheetHighlighter {
    fn language(&self) -> SharedString {
        LANGUAGE.into()
    }

    fn update(
        &mut self,
        _edit: Option<InputEdit>,
        text: &Rope,
        _folding: bool,
        _window: &mut Window,
        _cx: &mut Context<'_, EditorState>,
    ) {
        self.sheet.borrow_mut().set_text(&text.to_string());
    }

    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        self.sheet
            .borrow()
            .runs(range)
            .into_iter()
            .map(|(run, kind)| {
                (
                    run,
                    kind.and_then(|kind| resolver.style(kind_name(kind)))
                        .unwrap_or_default(),
                )
            })
            .collect()
    }

    fn fold_ranges(&self, _text: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use napkin_core::Span;

    use super::*;

    fn sheet(text: &str) -> Sheet {
        let mut sheet = Sheet::new(Arc::new(RwLock::new(Engine::new())));
        sheet.set_text(text);
        sheet
    }

    #[test]
    fn runs_cover_the_requested_range_without_gaps() {
        let sheet = sheet("12\nhello\n 3.5 \n");
        let range = 0..sheet.text().len();
        let runs = sheet.runs(&range);
        let mut cursor = range.start;
        for (run, _) in &runs {
            assert_eq!(run.start, cursor, "runs {runs:?}");
            assert!(run.end > run.start, "runs {runs:?}");
            cursor = run.end;
        }
        assert_eq!(cursor, range.end, "runs {runs:?}");
        assert!(runs.contains(&(0..2, Some(SpanKind::Number))), "runs {runs:?}");
        assert!(runs.contains(&(10..13, Some(SpanKind::Number))), "runs {runs:?}");
    }

    #[test]
    fn runs_clip_to_a_partial_range() {
        let sheet = sheet("1234");
        assert_eq!(sheet.runs(&(1..3)), vec![(1..3, Some(SpanKind::Number))]);
    }

    #[test]
    fn runs_tolerate_a_range_past_the_text() {
        let sheet = sheet("7");
        assert_eq!(
            sheet.runs(&(0..3)),
            vec![(0..1, Some(SpanKind::Number)), (1..3, None)]
        );
    }

    #[test]
    fn spans_off_char_boundaries_are_skipped() {
        let mut sheet = sheet("é1");
        sheet.lines[0].spans = vec![Span {
            range: 1..3,
            kind: SpanKind::Number,
        }];
        assert_eq!(sheet.runs(&(0..3)), vec![(0..3, None)]);
    }

    #[test]
    fn lines_and_answers_follow_the_text() {
        let sheet = sheet("1\nfoo\n2.5");
        assert_eq!(sheet.line_count(), 3);
        assert_eq!(sheet.line_text(1), "foo");
        assert_eq!(sheet.line_at(2), 1);
        assert_eq!(sheet.line_range(2), 6..9);
        assert_eq!(sheet.answer(0).map(|answer| answer.text.as_str()), Some("1"));
        assert!(sheet.answer(1).is_none());
        assert_eq!(sheet.answer(2).map(|answer| answer.text.as_str()), Some("2.5"));
    }
}
