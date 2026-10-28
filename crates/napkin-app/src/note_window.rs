//! The main window: title, the calculation editor with its result column, and the notes list.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::input::{EditorState, InputEvent};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Icon, Selectable as _, Sizable as _};
use gpui_kit::{
    AnyElement, AppContext as _, ClipboardItem, Context, Entity, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement as _, PathPromptOptions, Render, SharedString, Styled as _, Subscription,
    Task, Window, div, px,
};

use crate::app::{self, Napkin};
use crate::editor::{EditorArea, new_editor};
use crate::notes::{self, NoteStore};
use crate::notes_list::{NotesList, NotesListEvent};
use crate::platform::{self, PrintLine};
use crate::sheet::Sheet;
use crate::{ExtraIcon, settings::WindowFrame};

/// Height of the title area under the traffic lights.
const TITLE_HEIGHT: f32 = 38.;
/// Autosave waits this long after the last keystroke.
const SAVE_DELAY: Duration = Duration::from_millis(400);
/// How long "Copied" replaces a result.
const FLASH_DURATION: Duration = Duration::from_millis(900);
/// Window frame changes are saved after the window stops moving for this long.
const FRAME_SAVE_DELAY: Duration = Duration::from_millis(500);

/// A result showing "Copied" for a moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Flash {
    line: usize,
    generation: u64,
}

/// The main window's view.
pub(crate) struct NoteWindow {
    store: NoteStore,
    /// File name of the open note.
    current: String,
    sheet: Rc<RefCell<Sheet>>,
    editor: Entity<EditorState>,
    /// Text as last written to disk.
    saved_text: String,
    notes_list: Option<(Entity<NotesList>, Subscription)>,
    flash: Option<Flash>,
    flash_generation: u64,
    save_task: Option<Task<()>>,
    frame_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl NoteWindow {
    /// Opens `last_note` when it still exists, else the newest note, else a new one.
    pub(crate) fn new(last_note: Option<&str>, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let napkin = cx.global::<Napkin>();
        let store = napkin.note_store();
        let sheet = Rc::new(RefCell::new(Sheet::new(std::sync::Arc::clone(&napkin.engine))));
        let (current, text) = pick_note(&store, last_note);
        sheet.borrow_mut().set_text(&text);
        let editor = new_editor(&sheet, &text, window, cx);
        let subscriptions = vec![
            cx.subscribe_in(&editor, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.on_text_changed(window, cx);
                }
            }),
            cx.observe_window_bounds(window, Self::schedule_frame_save),
            cx.observe_window_appearance(window, |_, window, cx| {
                app::set_appearance(window.appearance(), cx);
            }),
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.save_now(cx);
                }
            }),
            cx.on_app_quit(|this, cx| {
                this.save_now(cx);
                async {}
            }),
        ];
        let view = Self {
            store,
            current,
            sheet,
            editor,
            saved_text: text,
            notes_list: None,
            flash: None,
            flash_generation: 0,
            save_task: None,
            frame_task: None,
            _subscriptions: subscriptions,
        };
        view.remember_current_note(cx);
        view.update_window_title(window);
        view
    }

    pub(crate) fn focus_editor(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.notes_list.is_none() {
            self.editor.update(cx, |editor, cx| editor.focus(window, cx));
        }
    }

    fn text(&self, cx: &gpui_kit::App) -> String {
        self.editor.read(cx).value().to_string()
    }

    fn on_text_changed(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = self.text(cx);
        self.sheet.borrow_mut().set_text(&text);
        self.flash = None;
        self.update_window_title(window);
        self.save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DELAY).await;
            let _ = this.update(cx, Self::save_now);
        }));
        cx.notify();
    }

    /// Writes the note when its text changed since the last save.
    pub(crate) fn save_now(&mut self, cx: &mut Context<'_, Self>) {
        self.save_task = None;
        let text = self.text(cx);
        if text == self.saved_text {
            return;
        }
        match self.store.save(&self.current, &text) {
            Ok(()) => self.saved_text = text,
            Err(error) => eprintln!("could not save the note: {error:#}"),
        }
    }

    /// Settings, rates or extensions changed: evaluate the open note again.
    pub(crate) fn reevaluate(&mut self, cx: &mut Context<'_, Self>) {
        self.sheet.borrow_mut().reevaluate();
        self.editor.update(cx, |_, cx| cx.notify());
        cx.notify();
    }

    fn title(&self) -> String {
        notes::title_for(&self.current, self.sheet.borrow().text())
    }

    fn update_window_title(&self, window: &mut Window) {
        window.set_window_title(&self.title());
    }

    fn remember_current_note(&self, cx: &mut Context<'_, Self>) {
        let napkin = cx.global::<Napkin>();
        let mut state = napkin.window_state();
        if state.last_note.as_deref() != Some(self.current.as_str()) {
            state.last_note = Some(self.current.clone());
            napkin.save_window_state(&state);
        }
    }

    fn schedule_frame_save(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let bounds = window.window_bounds().get_bounds();
        let frame = WindowFrame {
            x: f32::from(bounds.origin.x),
            y: f32::from(bounds.origin.y),
            width: f32::from(bounds.size.width),
            height: f32::from(bounds.size.height),
        };
        self.frame_task = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(FRAME_SAVE_DELAY).await;
            cx.update(|cx| {
                let napkin = cx.global::<Napkin>();
                let mut state = napkin.window_state();
                state.frame = Some(frame);
                napkin.save_window_state(&state);
            });
        }));
    }

    /// Switches to `file_name`, saving the current note first.
    fn open_note(&mut self, file_name: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_now(cx);
        let text = match self.store.read(file_name) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("could not open {file_name}: {error}");
                return;
            }
        };
        self.discard_if_empty(file_name);
        self.load(file_name.to_owned(), text, window, cx);
    }

    /// Shows `text` as the note `file_name`.
    fn load(&mut self, file_name: String, text: String, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.current = file_name;
        self.sheet.borrow_mut().set_text(&text);
        self.editor.update(cx, |editor, cx| {
            editor.set_value(text.clone(), window, cx);
            editor.set_selected_range(text.len()..text.len(), cx);
        });
        self.saved_text = text;
        self.flash = None;
        self.remember_current_note(cx);
        self.update_window_title(window);
        cx.notify();
    }

    /// Removes the open note when it is empty and we are moving to `next`.
    fn discard_if_empty(&self, next: &str) {
        if next != self.current
            && self.saved_text.trim().is_empty()
            && let Err(error) = self.store.delete(&self.current)
        {
            eprintln!("could not remove the empty note: {error:#}");
        }
    }

    pub(crate) fn new_note(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.close_notes_list(window, cx);
        self.save_now(cx);
        if self.text(cx).trim().is_empty() {
            self.focus_editor(window, cx);
            return;
        }
        match self.store.create("") {
            Ok(file_name) => {
                self.load(file_name, String::new(), window, cx);
                self.focus_editor(window, cx);
            }
            Err(error) => eprintln!("could not create a note: {error:#}"),
        }
    }

    /// Moves `step` notes down (positive) or up (negative) the notes list, wrapping around.
    pub(crate) fn step_note(&mut self, step: isize, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_now(cx);
        let notes = self.store.list();
        let Some(index) = notes.iter().position(|note| note.file_name == self.current) else {
            return;
        };
        let Some(count) = isize::try_from(notes.len()).ok().filter(|&count| count > 1) else {
            return;
        };
        let current = isize::try_from(index).unwrap_or_default();
        let next = usize::try_from((current + step).rem_euclid(count)).unwrap_or_default();
        let file_name = notes[next].file_name.clone();
        self.open_note(&file_name, window, cx);
        self.focus_editor(window, cx);
    }

    /// The notes folder changed: open the newest note there.
    pub(crate) fn reload_notes(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_now(cx);
        self.close_notes_list(window, cx);
        self.store = cx.global::<Napkin>().note_store();
        let (file_name, text) = pick_note(&self.store, None);
        self.load(file_name, text, window, cx);
    }

    pub(crate) fn toggle_notes_list(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.notes_list.is_some() {
            self.close_notes_list(window, cx);
            return;
        }
        self.save_now(cx);
        let notes = self.store.list();
        let list = cx.new(|cx| NotesList::new(notes, &self.current, window, cx));
        let subscription = cx.subscribe_in(&list, window, Self::on_notes_list_event);
        list.update(cx, |list, cx| list.focus(window, cx));
        self.notes_list = Some((list, subscription));
        cx.notify();
    }

    fn close_notes_list(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.notes_list.take().is_some() {
            self.focus_editor(window, cx);
            cx.notify();
        }
    }

    fn on_notes_list_event(
        &mut self,
        list: &Entity<NotesList>,
        event: &NotesListEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        match event {
            NotesListEvent::Open(file_name) => {
                self.close_notes_list(window, cx);
                self.open_note(file_name, window, cx);
                self.focus_editor(window, cx);
            }
            NotesListEvent::Dismiss => self.close_notes_list(window, cx),
            NotesListEvent::New => self.new_note(window, cx),
            NotesListEvent::Rename { file_name, title } => {
                self.save_now(cx);
                match self.store.rename(file_name, title) {
                    Ok(renamed) => {
                        if *file_name == self.current {
                            self.current.clone_from(&renamed);
                            self.remember_current_note(cx);
                            self.update_window_title(window);
                        }
                        let notes = self.store.list();
                        list.update(cx, |list, cx| {
                            list.set_notes(notes, &self.current, Some(&renamed), cx);
                        });
                    }
                    Err(error) => eprintln!("could not rename the note: {error:#}"),
                }
            }
            NotesListEvent::Delete(file_name) => {
                self.save_now(cx);
                if let Err(error) = self.store.delete(file_name) {
                    eprintln!("could not delete the note: {error:#}");
                    return;
                }
                if *file_name == self.current {
                    let (next, text) = pick_note(&self.store, None);
                    self.load(next, text, window, cx);
                }
                let notes = self.store.list();
                list.update(cx, |list, cx| list.set_notes(notes, &self.current, None, cx));
                list.update(cx, |list, cx| list.focus(window, cx));
            }
        }
    }

    pub(crate) fn import(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_now(cx);
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| this.import_file(&path, window, cx));
        })
        .detach();
    }

    /// Opens a `.numi` / `.txt` file as a new note named after the file.
    pub(crate) fn import_file(&mut self, path: &Path, window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => {
                eprintln!("could not import {}: {error}", path.display());
                return;
            }
        };
        self.save_now(cx);
        let created = self.store.create(&text).and_then(|file_name| {
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            self.store.rename(&file_name, name)
        });
        match created {
            Ok(file_name) => {
                self.discard_if_empty(&file_name);
                self.load(file_name, text, window, cx);
                self.focus_editor(window, cx);
            }
            Err(error) => eprintln!("could not import {}: {error:#}", path.display()),
        }
    }

    pub(crate) fn export(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_now(cx);
        let directory = dirs::document_dir().or_else(dirs::home_dir).unwrap_or_default();
        let name = format!("{}.{}", sanitize_file_name(&self.title()), notes::EXTENSION);
        let target = cx.prompt_for_new_path(&directory, Some(&name));
        let text = self.text(cx);
        cx.spawn_in(window, async move |_, _| {
            let Ok(Ok(Some(path))) = target.await else {
                return;
            };
            if let Err(error) = std::fs::write(&path, text) {
                eprintln!("could not export to {}: {error}", path.display());
            }
        })
        .detach();
    }

    pub(crate) fn print(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let lines: Vec<PrintLine> = {
            let sheet = self.sheet.borrow();
            (0..sheet.line_count())
                .map(|line| PrintLine {
                    input: sheet.line_text(line).to_owned(),
                    result: sheet.answer(line).map(|answer| answer.text.clone()),
                })
                .collect()
        };
        if let Err(error) = platform::print(&self.title(), &lines, window) {
            eprintln!("could not print: {error:#}");
        }
        let _ = cx;
    }

    /// `⇧⌘0`: wraps the selection, or the current line's expression, in parentheses.
    pub(crate) fn surround_with_parentheses(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let selection = self.editor.read(cx).selected_range();
        let range = if selection.is_empty() {
            let sheet = self.sheet.borrow();
            let line = sheet.line_at(selection.start);
            let line_range = sheet.line_range(line);
            let expression = expression_range(sheet.line_text(line));
            if expression.is_empty() {
                return;
            }
            line_range.start + expression.start..line_range.start + expression.end
        } else {
            selection
        };
        let text = self.text(cx);
        let Some(inner) = text.get(range.clone()) else {
            return;
        };
        let wrapped = format!("({inner})");
        self.editor.update(cx, |editor, cx| {
            editor.set_selected_range(range, cx);
            editor.replace(wrapped, window, cx);
        });
    }

    /// `⇧⌘C`: copies the result of the line with the caret.
    pub(crate) fn copy_current_line_result(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let cursor = self.editor.read(cx).cursor();
        let line = self.sheet.borrow().line_at(cursor);
        self.copy_result(line, cx);
    }

    /// Copies `line`'s result and flashes "Copied" in its place.
    fn copy_result(&mut self, line: usize, cx: &mut Context<'_, Self>) {
        let Some(text) = self.sheet.borrow().answer(line).map(|answer| answer.text.clone()) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.flash_generation += 1;
        let flash = Flash {
            line,
            generation: self.flash_generation,
        };
        self.flash = Some(flash);
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(FLASH_DURATION).await;
            let _ = this.update(cx, |this, cx| {
                if this.flash == Some(flash) {
                    this.flash = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// `⌥⌘⌫`: clears the note (undoable).
    pub(crate) fn delete_all(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let length = self.text(cx).len();
        self.editor.update(cx, |editor, cx| {
            editor.set_selected_range(0..length, cx);
            editor.replace("", window, cx);
        });
    }

    /// `⌥⌘C`: copies every line, with ` = result` after the lines that compute something.
    pub(crate) fn copy_all(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let sheet = self.sheet.borrow();
        let text: Vec<String> = (0..sheet.line_count())
            .map(|line| {
                let input = sheet.line_text(line).trim_end();
                match sheet.answer(line) {
                    Some(answer) => format!("{input} = {}", answer.text),
                    None => input.to_owned(),
                }
            })
            .collect();
        cx.write_to_clipboard(ClipboardItem::new_string(text.join("\n")));
    }

    fn render_title(&self, palette: &crate::theme::Palette, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let list_open = self.notes_list.is_some();
        div()
            .id("title-bar")
            .h(px(TITLE_HEIGHT))
            .flex_none()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .px(px(80.))
            .child(
                div()
                    .text_size(px(12.5))
                    .font_family(".SystemUIFont")
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(palette.muted)
                    .truncate()
                    .child(SharedString::from(self.title())),
            )
            .child(
                div().absolute().right(px(8.)).top(px(5.)).child(
                    Button::new("notes")
                        .ghost()
                        .small()
                        .icon(Icon::new(ExtraIcon::List).text_color(palette.muted))
                        .selected(list_open)
                        .tooltip("Notes (⌘L)")
                        .on_click(cx.listener(|this, _, window, cx| this.toggle_notes_list(window, cx))),
                ),
            )
    }
}

impl Render for NoteWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let napkin = cx.global::<Napkin>();
        let palette = napkin.palette();
        let font = napkin.editor_font();
        let view = cx.entity().downgrade();
        let area = EditorArea {
            editor: &self.editor,
            sheet: &self.sheet,
            palette: &palette,
            font,
            width: window.bounds().size.width,
            flash_line: self.flash.map(|flash| flash.line),
            on_copy: Rc::new(move |line, _window, cx| {
                let _ = view.update(cx, |this, cx| this.copy_result(line, cx));
            }),
        };
        let editor = area.render(window, cx).into_any_element();
        let overlay: Option<AnyElement> = self.notes_list.as_ref().map(|(list, _)| {
            div()
                .absolute()
                .top(px(TITLE_HEIGHT))
                .left_0()
                .right_0()
                .bottom_0()
                .child(list.clone())
                .into_any_element()
        });
        div()
            .id("napkin")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.background)
            .text_color(palette.text)
            .child(self.render_title(&palette, cx))
            .child(editor)
            .children(overlay)
    }
}

/// The note to show at start: `preferred` when it exists, else the newest, else a new one.
fn pick_note(store: &NoteStore, preferred: Option<&str>) -> (String, String) {
    if let Some(name) = preferred
        && let Ok(text) = store.read(name)
    {
        return (name.to_owned(), text);
    }
    if let Some(note) = store.list().into_iter().next() {
        return (note.file_name, note.text);
    }
    match store.create("") {
        Ok(name) => (name, String::new()),
        Err(error) => {
            eprintln!("could not create a note: {error:#}");
            ("untitled.numi".to_owned(), String::new())
        }
    }
}

/// The part of a line `⇧⌘0` wraps: without a leading label, a trailing `//` comment and outer
/// whitespace.
fn expression_range(line: &str) -> std::ops::Range<usize> {
    let mut end = line.find("//").unwrap_or(line.len());
    let mut start = label_end(&line[..end]).unwrap_or(0);
    let inner = &line[start..end];
    start += inner.len() - inner.trim_start().len();
    end = start + line[start..end].trim_end().len();
    start..end
}

/// The byte after a `Label:` prefix (a colon followed by whitespace, not between two digits).
fn label_end(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    bytes.iter().enumerate().find_map(|(index, &byte)| {
        let followed_by_space = bytes.get(index + 1).is_none_or(u8::is_ascii_whitespace);
        let between_digits = index > 0
            && bytes[index - 1].is_ascii_digit()
            && bytes.get(index + 1).is_some_and(u8::is_ascii_digit);
        (byte == b':' && followed_by_space && !between_digits).then_some(index + 1)
    })
}

fn sanitize_file_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|character| {
            if matches!(character, '/' | ':' | '\\') {
                '-'
            } else {
                character
            }
        })
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        notes::UNTITLED.to_owned()
    } else {
        cleaned.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped(line: &str) -> &str {
        &line[expression_range(line)]
    }

    #[test]
    fn the_expression_skips_labels_comments_and_spaces() {
        assert_eq!(wrapped("  20 inches in cm  "), "20 inches in cm");
        assert_eq!(wrapped("Price: $11 + $34.45"), "$11 + $34.45");
        assert_eq!(wrapped("2:30 pm + 2 hours"), "2:30 pm + 2 hours");
        assert_eq!(wrapped("5 + 5 // why"), "5 + 5");
        assert_eq!(wrapped("Total:"), "");
    }

    #[test]
    fn export_names_are_safe() {
        assert_eq!(sanitize_file_name("Trip: Rome/Paris"), "Trip- Rome-Paris");
        assert_eq!(sanitize_file_name("  "), notes::UNTITLED);
    }
}
