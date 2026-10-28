//! The calculation editor: gpui-base's code editor configured as a plain notepad, with the
//! engine's spans as syntax colours and the [`ResultsLayer`] laid over its right edge.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::base::input::{EditorState, InputEditorStyle};
use gpui_kit::{
    App, AppContext as _, Context, Edges, Entity, Font, FontFeatures, FontStyle, FontWeight, IntoElement,
    ParentElement as _, Pixels, Styled as _, Window, div, px,
};

use crate::results::{CopyListener, ResultFont, ResultsLayer, ResultsStyle, widest_result};
use crate::settings::FontChoice;
use crate::sheet::{LANGUAGE, Sheet, SheetHighlighter};
use crate::theme::Palette;

/// Space left of the text.
const PAD_LEFT: Pixels = px(22.);
/// Space above the first line, below the title bar.
const PAD_TOP: Pixels = px(2.);
/// Space below the last line.
const PAD_BOTTOM: Pixels = px(18.);
/// Gap between the text and the result column.
const COLUMN_GAP: Pixels = px(28.);
/// Space right of the result column.
const PAD_RIGHT: Pixels = px(22.);
/// The result column never takes more than this share of the editor width.
const MAX_COLUMN_SHARE: f32 = 0.5;
/// Line height as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.65;

/// Font settings of the editor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EditorFont {
    pub(crate) choice: FontChoice,
    pub(crate) size: f32,
}

impl EditorFont {
    pub(crate) fn font(self) -> Font {
        Font {
            family: self.choice.family().into(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        }
    }

    pub(crate) fn line_height(self) -> Pixels {
        px((self.size * LINE_HEIGHT).round())
    }
}

/// Creates the editor state bound to `sheet`.
pub(crate) fn new_editor(
    sheet: &Rc<RefCell<Sheet>>,
    text: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<EditorState> {
    let sheet = Rc::clone(sheet);
    cx.new(|cx| {
        let mut state = EditorState::new(window, cx)
            .language(LANGUAGE)
            .line_number(false)
            .folding(false)
            .indent_guides(false)
            .auto_close(false)
            .smart_indent(false)
            .searchable(false)
            .soft_wrap(true)
            .context_menu(true)
            .placeholder("Type a calculation, like 20 inches in cm");
        state.set_highlighter_factory(
            Rc::new(move |_| Some(Box::new(SheetHighlighter::new(Rc::clone(&sheet))))),
            cx,
        );
        state.set_value(text.to_owned(), window, cx);
        // A scratchpad continues where it ends.
        state.set_selected_range(text.len()..text.len(), cx);
        state
    })
}

/// Applies colours and the result column width to the editor before it renders.
pub(crate) fn style_editor(
    editor: &Entity<EditorState>,
    palette: &Palette,
    column_width: Pixels,
    cx: &mut App,
) {
    editor.update(cx, |state, _| {
        state.set_editor_style(InputEditorStyle {
            foreground: palette.text,
            muted_foreground: palette.muted,
            background: palette.background,
            border: palette.border,
            selection: palette.selection,
            caret: palette.caret,
            highlight_styles: palette.highlight_styles(),
            ..InputEditorStyle::default()
        });
        state.set_editor_paddings(Edges {
            top: PAD_TOP,
            right: column_width + COLUMN_GAP + PAD_RIGHT,
            bottom: PAD_BOTTOM,
            left: PAD_LEFT,
        });
    });
}

/// Everything the editor area needs to render one frame.
pub(crate) struct EditorArea<'a> {
    pub(crate) editor: &'a Entity<EditorState>,
    pub(crate) sheet: &'a Rc<RefCell<Sheet>>,
    pub(crate) palette: &'a Palette,
    pub(crate) font: EditorFont,
    pub(crate) width: Pixels,
    pub(crate) flash_line: Option<usize>,
    pub(crate) on_copy: CopyListener,
}

impl EditorArea<'_> {
    /// Renders the editor with the result column over its right edge.
    pub(crate) fn render<V: 'static>(self, window: &mut Window, cx: &mut Context<'_, V>) -> impl IntoElement {
        let result_font = ResultFont {
            font: self.font.font(),
            size: px(self.font.size),
        };
        let widest = widest_result(&self.sheet.borrow(), &result_font, window);
        let max_column = (self.width - PAD_LEFT - PAD_RIGHT - COLUMN_GAP) * MAX_COLUMN_SHARE;
        let column_width = widest.min(max_column).max(px(0.));
        style_editor(self.editor, self.palette, column_width, cx);
        let style = ResultsStyle {
            font: result_font,
            color: self.palette.result,
            hover_background: self.palette.result_hover,
            flash_color: self.palette.accent,
            column_width,
            right_inset: PAD_RIGHT,
        };
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .font_family(self.font.choice.family())
            .text_size(px(self.font.size))
            .line_height(self.font.line_height())
            .text_color(self.palette.text)
            .child(self.editor.clone())
            .child(ResultsLayer::new(
                self.editor.clone(),
                Rc::clone(self.sheet),
                style,
                self.flash_line,
                self.on_copy,
            ))
    }
}
