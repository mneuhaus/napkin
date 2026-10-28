//! The result column: each line's answer drawn right-aligned on the first visual row of its
//! line, hoverable, and clickable to copy.
//!
//! The layer is a sibling painted after the editor. The editor publishes its line layout while
//! it paints, so by the time this layer paints, `EditorState::range_to_bounds` describes the
//! current frame: scrolling and soft wrap line up without a frame of lag.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::base::input::EditorState;
use gpui_kit::{
    App, Bounds, ContentMask, CursorStyle, DispatchPhase, Element, ElementId, Entity, Font, GlobalElementId,
    Hitbox, HitboxBehavior, Hsla, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, Pixels, Position, ShapedLine, SharedString, Style, TextAlign, TextRun, Window, fill,
    point, px, relative, size,
};

use crate::sheet::Sheet;

/// Horizontal padding of the hover highlight around a result.
const HOVER_PAD_X: Pixels = px(6.);
/// Shown in place of a result right after it was copied.
const COPIED: &str = "Copied";

/// Called with the line whose result was clicked.
pub(crate) type CopyListener = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Colours and geometry of the result column.
#[derive(Debug, Clone)]
pub(crate) struct ResultsStyle {
    pub(crate) font: ResultFont,
    pub(crate) color: Hsla,
    pub(crate) hover_background: Hsla,
    pub(crate) flash_color: Hsla,
    /// Width of the column the results are right-aligned in; longer results are truncated.
    pub(crate) column_width: Pixels,
    /// Space between the column's right edge and the layer's right edge.
    pub(crate) right_inset: Pixels,
}

/// The result overlay laid over the editor. See the module docs for how it stays aligned.
pub(crate) struct ResultsLayer {
    editor: Entity<EditorState>,
    sheet: Rc<RefCell<Sheet>>,
    style: ResultsStyle,
    /// The line currently showing the "Copied" flash.
    flash_line: Option<usize>,
    on_copy: CopyListener,
}

impl ResultsLayer {
    pub(crate) fn new(
        editor: Entity<EditorState>,
        sheet: Rc<RefCell<Sheet>>,
        style: ResultsStyle,
        flash_line: Option<usize>,
        on_copy: CopyListener,
    ) -> Self {
        Self {
            editor,
            sheet,
            style,
            flash_line,
            on_copy,
        }
    }
}

/// A result placed for this frame.
struct PlacedResult {
    line: usize,
    shaped: ShapedLine,
    origin: gpui_kit::Point<Pixels>,
    /// The clickable area.
    target: Bounds<Pixels>,
}

impl IntoElement for ResultsLayer {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ResultsLayer {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style {
            position: Position::Absolute,
            ..Style::default()
        };
        style.inset.top = px(0.).into();
        style.inset.left = px(0.).into();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let placed = self.place(bounds, window, cx);
        let Some(line_height) = self.editor.read(cx).line_height() else {
            return;
        };
        let mouse = window.mouse_position();
        let hovered = hitbox
            .is_hovered(window)
            .then(|| placed.iter().position(|result| result.target.contains(&mouse)))
            .flatten();

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for (index, result) in placed.iter().enumerate() {
                if hovered == Some(index) {
                    window.paint_quad(fill(result.target, self.style.hover_background).corner_radii(px(5.)));
                }
                if let Err(error) =
                    result
                        .shaped
                        .paint(result.origin, line_height, TextAlign::Left, None, window, cx)
                {
                    eprintln!("could not paint a result: {error:#}");
                }
            }
        });

        if hovered.is_some() {
            window.set_cursor_style(CursorStyle::PointingHand, hitbox);
        }

        let targets: Rc<[(Bounds<Pixels>, usize)]> =
            placed.iter().map(|result| (result.target, result.line)).collect();
        let hitbox = hitbox.clone();
        let on_copy = Rc::clone(&self.on_copy);
        let click_targets = Rc::clone(&targets);
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || event.button != MouseButton::Left || event.modifiers.shift {
                return;
            }
            if !hitbox.is_hovered_at(event.position, window) {
                return;
            }
            if let Some(&(_, line)) = click_targets
                .iter()
                .find(|(target, _)| target.contains(&event.position))
            {
                on_copy(line, window, cx);
                cx.stop_propagation();
            }
        });
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            let now = targets
                .iter()
                .position(|(target, _)| target.contains(&event.position));
            if now != hovered {
                window.refresh();
            }
        });
    }
}

impl ResultsLayer {
    /// Shapes and positions the results of the lines the editor shows this frame.
    fn place(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &App) -> Vec<PlacedResult> {
        let editor = self.editor.read(cx);
        let (Some(rows), Some(line_height)) = (editor.visible_row_range(), editor.line_height()) else {
            return Vec::new();
        };
        let sheet = self.sheet.borrow();
        let right = bounds.right() - self.style.right_inset;
        let mut placed = Vec::new();
        for line in rows {
            let flashing = self.flash_line == Some(line);
            let text: SharedString = match sheet.answer(line) {
                _ if flashing => COPIED.into(),
                Some(answer) => answer.text.clone().into(),
                None => continue,
            };
            let start = sheet.line_range(line).start;
            let Some(row) = editor.range_to_bounds(&(start..start)) else {
                continue;
            };
            let color = if flashing {
                self.style.flash_color
            } else {
                self.style.color
            };
            let shaped = shape_fitting(&text, color, &self.style.font, self.style.column_width, window);
            let origin = point(right - shaped.width, row.origin.y);
            let target = Bounds::new(
                point(origin.x - HOVER_PAD_X, origin.y),
                size(shaped.width + HOVER_PAD_X * 2., line_height),
            );
            placed.push(PlacedResult {
                line,
                shaped,
                origin,
                target,
            });
        }
        placed
    }
}

/// The font results are drawn in.
#[derive(Debug, Clone)]
pub(crate) struct ResultFont {
    pub(crate) font: Font,
    pub(crate) size: Pixels,
}

impl ResultFont {
    fn shape(&self, text: SharedString, color: Hsla, window: &Window) -> ShapedLine {
        let run = TextRun {
            len: text.len(),
            font: self.font.clone(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        window.text_system().shape_line(text, self.size, &[run], None)
    }
}

/// Shapes `text`, cut with an ellipsis when wider than `max_width`.
fn shape_fitting(
    text: &SharedString,
    color: Hsla,
    font: &ResultFont,
    max_width: Pixels,
    window: &Window,
) -> ShapedLine {
    let full = font.shape(text.clone(), color, window);
    if full.width <= max_width {
        return full;
    }
    let chars: Vec<char> = text.chars().collect();
    let truncated = |keep: usize| -> SharedString {
        chars[..keep]
            .iter()
            .chain(['…'].iter())
            .collect::<String>()
            .into()
    };
    let (mut keep_low, mut keep_high) = (0, chars.len());
    while keep_low < keep_high {
        let keep = (keep_low + keep_high).div_ceil(2);
        if font.shape(truncated(keep), color, window).width <= max_width {
            keep_low = keep;
        } else {
            keep_high = keep - 1;
        }
    }
    font.shape(truncated(keep_low), color, window)
}

/// Width of the widest result in the document.
pub(crate) fn widest_result(sheet: &Sheet, font: &ResultFont, window: &Window) -> Pixels {
    sheet
        .answers()
        .flatten()
        .map(|answer| {
            font.shape(answer.text.clone().into(), Hsla::default(), window)
                .width
        })
        .fold(px(0.), Pixels::max)
}
