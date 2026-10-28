//! Napkin: a calculator notepad. Type math in plain words, read the results on the right.

mod editor;
mod paths;
mod results;
mod settings;
mod sheet;
mod theme;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use gpui_kit::base::input::{EditorState, InputEvent};
use gpui_kit::{
    App, AppContext as _, Bounds, ClipboardItem, Context, Entity, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Styled as _, Subscription, TitlebarOptions, Window,
    WindowBounds, WindowOptions, div, point, px, size,
};
use napkin_core::Engine;

use crate::editor::{EditorArea, EditorFont, new_editor};
use crate::settings::{DEFAULT_FONT_SIZE, FontChoice};
use crate::sheet::Sheet;
use crate::theme::ThemeChoice;

const SAMPLE: &str = "# Trip\n12\n3.5\nnot a number\n1000000\n0.25\nA long line of prose that should soft wrap before it reaches the result column on the right side of the window, keeping the result aligned with the first row\n42\n// a comment\n7";

struct NoteWindow {
    sheet: Rc<RefCell<Sheet>>,
    editor: Entity<EditorState>,
    flash: Option<(usize, u64)>,
    flash_generation: u64,
    _subscriptions: Vec<Subscription>,
}

impl NoteWindow {
    fn new(text: &str, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let engine = Arc::new(RwLock::new(Engine::new()));
        let sheet = Rc::new(RefCell::new(Sheet::new(engine)));
        sheet.borrow_mut().set_text(text);
        let editor = new_editor(&sheet, text, window, cx);
        let subscriptions = vec![
            cx.subscribe_in(&editor, window, |this, editor, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sheet.borrow_mut().set_text(&editor.read(cx).value());
                    cx.notify();
                }
            }),
        ];
        Self {
            sheet,
            editor,
            flash: None,
            flash_generation: 0,
            _subscriptions: subscriptions,
        }
    }

    fn copy_result(&mut self, line: usize, cx: &mut Context<'_, Self>) {
        let Some(text) = self.sheet.borrow().answer(line).map(|answer| answer.text.clone()) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.flash_generation += 1;
        let generation = self.flash_generation;
        self.flash = Some((line, generation));
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(900)).await;
            let _ = this.update(cx, |this, cx| {
                if this.flash.is_some_and(|(_, shown)| shown == generation) {
                    this.flash = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

impl Render for NoteWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let palette = ThemeChoice::System.palette(window.appearance());
        let width = window.bounds().size.width;
        let view = cx.entity().downgrade();
        let area = EditorArea {
            editor: &self.editor,
            sheet: &self.sheet,
            palette: &palette,
            font: EditorFont {
                choice: FontChoice::System,
                size: DEFAULT_FONT_SIZE,
            },
            width,
            flash_line: self.flash.map(|(line, _)| line),
            on_copy: Rc::new(move |line, _window, cx| {
                let _ = view.update(cx, |this, cx| this.copy_result(line, cx));
            }),
        };
        div()
            .id("napkin")
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.background)
            .child(
                div()
                    .h(px(38.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(palette.muted)
                    .child("Trip"),
            )
            .child(area.render(window, cx))
    }
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        let bounds = Bounds::centered(None, size(px(560.), px(440.)), cx);
        let opened = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Napkin".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(13.), px(13.))),
                }),
                window_min_size: Some(size(px(320.), px(200.))),
                focus: std::env::var_os("NAPKIN_BACKGROUND").is_none(),
                ..Default::default()
            },
            cx,
            |window, cx| {
                let text = std::env::args()
                    .nth(1)
                    .and_then(|path| std::fs::read_to_string(path).ok())
                    .unwrap_or_else(|| SAMPLE.to_owned());
                let view = cx.new(|cx| NoteWindow::new(&text, window, cx));
                let editor = view.read(cx).editor.clone();
                editor.update(cx, |editor, cx| editor.focus(window, cx));
                view
            },
        );
        if let Err(error) = opened {
            eprintln!("could not open the window: {error:#}");
            cx.quit();
        }
        if std::env::var_os("NAPKIN_BACKGROUND").is_none() {
            cx.activate(true);
        }
    });
}
