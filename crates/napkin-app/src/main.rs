//! Napkin: a calculator notepad. Type math in plain words, read the results on the right.
//!
//! STUB: a single window that proves the GPUI setup builds; the editor, notes, themes, settings,
//! menu bar item and global hotkey get added.

use gpui_kit::{
    App, AppContext as _, Bounds, Context, IntoElement, ParentElement as _, Render, Styled as _, TitlebarOptions,
    Window, WindowBounds, WindowOptions, div, px, size,
};

struct Napkin;

impl Render for Napkin {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().size_full().flex().items_center().justify_center().child("Napkin")
    }
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        let bounds = Bounds::centered(None, size(px(560.), px(440.)), cx);
        let opened = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("Napkin".into()), ..Default::default() }),
                ..Default::default()
            },
            cx,
            |_window, cx| cx.new(|_| Napkin),
        );
        if let Err(error) = opened {
            eprintln!("could not open the window: {error:#}");
            cx.quit();
        }
        cx.activate(true);
    });
}
