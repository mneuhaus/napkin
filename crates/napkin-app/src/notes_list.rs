//! The notes list (`⌘L`): search, switch, rename and delete notes.

use std::time::SystemTime;

use gpui_kit::base::input::{Escape, InputEvent, InputState, MoveDown, MoveUp};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AppContext as _, BoxShadow, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle, FontWeight,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _, PromptLevel, Render,
    ScrollHandle, SharedString, Stateful, StatefulInteractiveElement as _, Styled, Subscription, Window, div,
    hsla, point, px,
};

use crate::ExtraIcon;
use crate::app::Napkin;
use crate::notes::{self, Note};
use crate::theme::Palette;

gpui_kit::actions!(notes_list, [RenameSelected]);

/// Key context of the list.
const CONTEXT: &str = "NotesList";
/// Width of the panel.
const PANEL_WIDTH: f32 = 440.;

pub(crate) fn bind_keys(cx: &mut gpui_kit::App) {
    cx.bind_keys([KeyBinding::new("cmd-r", RenameSelected, Some(CONTEXT))]);
}

/// What the list asks the window to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NotesListEvent {
    Open(String),
    Dismiss,
    New,
    Rename { file_name: String, title: String },
    Delete(String),
}

struct Renaming {
    file_name: String,
    input: Entity<InputState>,
    _subscription: Subscription,
}

pub(crate) struct NotesList {
    notes: Vec<Note>,
    current: String,
    query: Entity<InputState>,
    /// Index into the filtered notes.
    selected: usize,
    renaming: Option<Renaming>,
    scroll: ScrollHandle,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NotesListEvent> for NotesList {}

impl NotesList {
    pub(crate) fn new(
        notes: Vec<Note>,
        current: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search notes"));
        let subscriptions = vec![cx.subscribe_in(
            &query,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    this.selected = 0;
                    this.scroll.scroll_to_item(0);
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.open_selected(window, cx),
                InputEvent::Focus | InputEvent::Blur => {}
            },
        )];
        let selected = notes
            .iter()
            .position(|note| note.file_name == current)
            .unwrap_or(0);
        let scroll = ScrollHandle::new();
        scroll.scroll_to_item(selected);
        Self {
            notes,
            current: current.to_owned(),
            query,
            selected,
            renaming: None,
            scroll,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn focus(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        match &self.renaming {
            Some(renaming) => renaming.input.update(cx, |input, cx| input.focus(window, cx)),
            None => self.query.update(cx, |query, cx| query.focus(window, cx)),
        }
    }

    /// Replaces the notes (after a rename or delete), keeping `select` selected when given.
    pub(crate) fn set_notes(
        &mut self,
        notes: Vec<Note>,
        current: &str,
        select: Option<&str>,
        cx: &mut Context<'_, Self>,
    ) {
        self.notes = notes;
        current.clone_into(&mut self.current);
        self.renaming = None;
        let filtered = self.filtered(cx);
        let wanted = select.unwrap_or(current);
        self.selected = filtered
            .iter()
            .position(|&index| self.notes[index].file_name == wanted)
            .unwrap_or(self.selected.min(filtered.len().saturating_sub(1)));
        cx.notify();
    }

    /// Indices of the notes matching the search, in list order.
    fn filtered(&self, cx: &gpui_kit::App) -> Vec<usize> {
        let query = self.query.read(cx).value().trim().to_lowercase();
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, note)| {
                query.is_empty()
                    || note.title.to_lowercase().contains(&query)
                    || note.text.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn selected_note(&self, cx: &gpui_kit::App) -> Option<&Note> {
        self.filtered(cx)
            .get(self.selected)
            .map(|&index| &self.notes[index])
    }

    fn move_selection(&mut self, step: isize, cx: &mut Context<'_, Self>) {
        let count = self.filtered(cx).len();
        if count == 0 {
            return;
        }
        let last = count - 1;
        self.selected = if step < 0 {
            self.selected.saturating_sub(1)
        } else {
            (self.selected + 1).min(last)
        };
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }

    fn open_selected(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(note) = self.selected_note(cx) {
            cx.emit(NotesListEvent::Open(note.file_name.clone()));
        }
    }

    fn start_rename(&mut self, file_name: String, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(note) = self.notes.iter().find(|note| note.file_name == file_name) else {
            return;
        };
        let title = if note.is_renamed() {
            note.title.clone()
        } else {
            String::new()
        };
        let placeholder = notes::first_line_title(&note.text);
        let input = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder(placeholder);
            input.set_value(title, window, cx);
            input
        });
        let subscription = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                let title = input.read(cx).value().to_string();
                if let Some(renaming) = this.renaming.take() {
                    cx.emit(NotesListEvent::Rename {
                        file_name: renaming.file_name,
                        title,
                    });
                }
                this.focus(window, cx);
            }
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.renaming = Some(Renaming {
            file_name,
            input,
            _subscription: subscription,
        });
        cx.notify();
    }

    fn confirm_delete(&mut self, file_name: String, window: &mut Window, cx: &mut Context<'_, Self>) {
        let title = self
            .notes
            .iter()
            .find(|note| note.file_name == file_name)
            .map_or_else(String::new, |note| note.title.clone());
        let answer = window.prompt(
            PromptLevel::Warning,
            &format!("Delete “{title}”?"),
            Some("The note moves to the Trash."),
            &["Delete", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                let _ = this.update(cx, |_, cx| cx.emit(NotesListEvent::Delete(file_name)));
            }
        })
        .detach();
    }

    fn on_escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.renaming.take().is_some() {
            self.focus(window, cx);
            cx.notify();
        } else {
            cx.emit(NotesListEvent::Dismiss);
        }
    }

    fn render_row(
        &self,
        position: usize,
        note: &Note,
        palette: &Palette,
        cx: &mut Context<'_, Self>,
    ) -> Stateful<Div> {
        let selected = position == self.selected;
        let file_name = note.file_name.clone();
        let renaming = self
            .renaming
            .as_ref()
            .filter(|renaming| renaming.file_name == note.file_name);
        let group = SharedString::from(format!("note-row-{position}"));
        let weight = if note.file_name == self.current {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        };
        let body = if let Some(renaming) = renaming {
            div().child(Input::new(&renaming.input).small())
        } else {
            row_text(note, weight, palette)
        };
        div()
            .id(SharedString::from(format!("note-{position}")))
            .group(group.clone())
            .mx(px(6.))
            .px(px(10.))
            .py(px(7.))
            .rounded(px(7.))
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_pointer()
            .when(selected, |row| row.bg(palette.result_hover))
            .hover(|row| row.bg(palette.result_hover.opacity(0.6)))
            .on_click(cx.listener({
                let file_name = file_name.clone();
                move |this, _: &ClickEvent, _, cx| {
                    this.selected = position;
                    cx.emit(NotesListEvent::Open(file_name.clone()));
                }
            }))
            .child(body.flex_1().min_w_0())
            .child(self.row_actions(position, file_name, group, palette, cx))
    }

    /// Rename and delete buttons, shown on hover and for the selected row.
    fn row_actions(
        &self,
        position: usize,
        file_name: String,
        group: SharedString,
        palette: &Palette,
        cx: &mut Context<'_, Self>,
    ) -> Div {
        let rename = {
            let file_name = file_name.clone();
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.start_rename(file_name.clone(), window, cx);
            })
        };
        let delete = cx.listener(move |this, _: &ClickEvent, window, cx| {
            this.confirm_delete(file_name.clone(), window, cx);
        });
        div()
            .flex_none()
            .flex()
            .gap(px(2.))
            .invisible()
            .group_hover(group, Styled::visible)
            .when(position == self.selected, Styled::visible)
            .child(
                Button::new(SharedString::from(format!("rename-{position}")))
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(ExtraIcon::Pencil).text_color(palette.muted))
                    .tooltip("Rename (⌘R)")
                    .on_click(rename),
            )
            .child(
                Button::new(SharedString::from(format!("delete-{position}")))
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(ExtraIcon::Trash).text_color(palette.muted))
                    .tooltip("Delete")
                    .on_click(delete),
            )
    }

    fn search_bar(&self, palette: &Palette, cx: &mut Context<'_, Self>) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .p(px(10.))
            .border_b_1()
            .border_color(palette.border)
            .child(
                div().flex_1().child(
                    Input::new(&self.query)
                        .prefix(Icon::new(ExtraIcon::Search).text_color(palette.muted))
                        .cleanable(true),
                ),
            )
            .child(
                Button::new("new-note")
                    .ghost()
                    .small()
                    .icon(Icon::new(ExtraIcon::Plus).text_color(palette.muted))
                    .tooltip("New note (⌘N)")
                    .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(NotesListEvent::New))),
            )
    }
}

/// Title, date and preview of a note.
fn row_text(note: &Note, weight: FontWeight, palette: &Palette) -> Div {
    let preview = preview(note);
    let title = div()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(13.5))
                .font_weight(weight)
                .text_color(palette.text)
                .child(SharedString::from(note.title.clone())),
        )
        .child(
            div()
                .flex_none()
                .text_size(px(11.5))
                .text_color(palette.muted)
                .child(SharedString::from(relative_date(note.modified))),
        );
    div().flex().flex_col().gap(px(2.)).child(title).child(
        div()
            .truncate()
            .text_size(px(12.))
            .text_color(palette.muted)
            .child(SharedString::from(if preview.is_empty() {
                " ".to_owned()
            } else {
                preview
            })),
    )
}

fn footer(palette: &Palette) -> Div {
    div()
        .px(px(14.))
        .py(px(7.))
        .border_t_1()
        .border_color(palette.border)
        .text_size(px(11.))
        .text_color(palette.muted)
        .child("↑↓ select   ↵ open   ⌘R rename   esc close")
}

fn panel_shadow(palette: &Palette) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hsla(0., 0., 0., if palette.dark { 0.45 } else { 0.16 }),
        offset: point(px(0.), px(10.)),
        blur_radius: px(28.),
        spread_radius: px(0.),
        inset: false,
    }]
}

impl Render for NotesList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let palette = cx.global::<Napkin>().palette();
        let filtered = self.filtered(cx);
        let rows: Vec<_> = filtered
            .iter()
            .enumerate()
            .map(|(position, &index)| {
                let note = self.notes[index].clone();
                self.render_row(position, &note, &palette, cx)
            })
            .collect();
        let empty = rows.is_empty().then(|| {
            div()
                .px(px(16.))
                .py(px(14.))
                .text_size(px(12.5))
                .text_color(palette.muted)
                .child("No matching notes")
        });
        let width = px(PANEL_WIDTH).min(window.bounds().size.width - px(32.));
        let backdrop = if palette.dark {
            hsla(0., 0., 0., 0.35)
        } else {
            hsla(0., 0., 0., 0.12)
        };
        let panel = div()
            .id("notes-panel")
            .w(width)
            .max_h_full()
            .flex()
            .flex_col()
            .bg(palette.panel)
            .border_1()
            .border_color(palette.border)
            .rounded(px(12.))
            .shadow(panel_shadow(&palette))
            .overflow_hidden()
            // Clicks inside the panel must not reach the backdrop.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(self.search_bar(&palette, cx))
            .child(
                div()
                    .id("notes-rows")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .py(px(6.))
                    .children(empty)
                    .children(rows),
            )
            .child(footer(&palette));
        div()
            .id("notes-backdrop")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .justify_center()
            .items_start()
            .pt(px(4.))
            .pb(px(24.))
            .bg(backdrop)
            .on_action(cx.listener(|this, _: &MoveUp, _, cx| this.move_selection(-1, cx)))
            .on_action(cx.listener(|this, _: &MoveDown, _, cx| this.move_selection(1, cx)))
            .on_action(cx.listener(Self::on_escape))
            .on_action(cx.listener(|this, _: &RenameSelected, window, cx| {
                if let Some(file_name) = this.selected_note(cx).map(|note| note.file_name.clone()) {
                    this.start_rename(file_name, window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.emit(NotesListEvent::Dismiss)),
            )
            .child(panel)
    }
}

/// The second non-empty line of a note, as a hint of what is in it.
fn preview(note: &Note) -> String {
    let mut lines = note.text.lines().map(str::trim).filter(|line| !line.is_empty());
    if !note.is_renamed() {
        lines.next();
    }
    lines.next().map(notes::first_line_title).unwrap_or_default()
}

/// "13:05" for today, "Yesterday", "Mon" within a week, else "28 Oct 2026".
fn relative_date(time: SystemTime) -> String {
    let time: chrono::DateTime<chrono::Local> = time.into();
    let today = chrono::Local::now().date_naive();
    let date = time.date_naive();
    match (today - date).num_days() {
        0 => time.format("%H:%M").to_string(),
        1 => "Yesterday".to_owned(),
        2..=6 => time.format("%a").to_string(),
        _ => time.format("%-d %b %Y").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(file_name: &str, text: &str) -> Note {
        Note {
            file_name: file_name.into(),
            title: notes::title_for(file_name, text),
            text: text.into(),
            created: SystemTime::UNIX_EPOCH,
            modified: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn previews_skip_the_line_used_as_title() {
        assert_eq!(
            preview(&note("note-20261028-130000.numi", "# Trip\n\nHotel: 3 * 89")),
            "Hotel: 3 * 89"
        );
        assert_eq!(preview(&note("Budget.numi", "Rent: 900\nFood: 300")), "Rent: 900");
        assert_eq!(preview(&note("note-20261028-130000.numi", "only")), "");
    }

    #[test]
    fn today_shows_the_time() {
        assert_eq!(relative_date(SystemTime::now()).len(), 5);
    }
}
