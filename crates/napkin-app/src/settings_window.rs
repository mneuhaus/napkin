//! The settings window (`⌘,`). Every change is saved and applied immediately.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{IndexPath, Selectable as _, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AnyWindowHandle, App, AppContext as _, ClickEvent, Context, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, PathPromptOptions, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, TitlebarOptions, Window, WindowBounds,
    WindowOptions, div, px, size,
};
use napkin_core::{AngleUnit, NumberStyle, TimeFormat};

use crate::actions::{DecreaseFontSize, IncreaseFontSize};
use crate::app::{self, Napkin};
use crate::hotkey;
use crate::settings::{AppSettings, DEFAULT_HOTKEY, FontChoice};
use crate::theme::ThemeChoice;

/// Highest precision the settings offer, as in Numi.
const MAX_PRECISION: u8 = 10;

type Choice = Entity<SelectState<SearchableVec<SharedString>>>;

/// Opens the settings window.
pub(crate) fn open(focus: bool, cx: &mut App) -> anyhow::Result<AnyWindowHandle> {
    let bounds = gpui_kit::Bounds::centered(None, size(px(520.), px(640.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("Napkin Settings".into()),
            ..Default::default()
        }),
        window_min_size: Some(size(px(440.), px(420.))),
        is_minimizable: false,
        focus,
        ..Default::default()
    };
    let (handle, _) = gpui_kit::open_window(options, cx, |window, cx| {
        window.on_window_should_close(cx, |_, cx| {
            app::forget_settings_window(cx);
            true
        });
        cx.new(|cx| SettingsWindow::new(window, cx))
    })?;
    Ok(handle)
}

pub(crate) struct SettingsWindow {
    theme: Choice,
    font: Choice,
    precision: Choice,
    number_style: Choice,
    angle_unit: Choice,
    time_format: Choice,
    recording_hotkey: Option<Subscription>,
    _subscriptions: Vec<Subscription>,
}

const NUMBER_STYLES: [(NumberStyle, &str); 5] = [
    (NumberStyle::English, "English  1,234.56"),
    (NumberStyle::German, "German  1.234,56"),
    (NumberStyle::Swiss, "Swiss  1'234.56"),
    (NumberStyle::French, "French  1 234,56"),
    (NumberStyle::Plain, "Plain  1234.56"),
];
const ANGLE_UNITS: [(AngleUnit, &str); 2] =
    [(AngleUnit::Degrees, "Degrees"), (AngleUnit::Radians, "Radians")];
const TIME_FORMATS: [(TimeFormat, &str); 2] = [
    (TimeFormat::H24, "24-hour  14:30"),
    (TimeFormat::H12, "12-hour  2:30 pm"),
];

impl SettingsWindow {
    fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let settings = cx.global::<Napkin>().settings.clone();
        let mut subscriptions = Vec::new();

        let theme = choice(
            ThemeChoice::ALL.map(ThemeChoice::label).as_slice(),
            settings.theme.label(),
            window,
            cx,
        );
        subscriptions.push(on_choice(&theme, window, cx, |label, settings| {
            if let Some(theme) = ThemeChoice::ALL.into_iter().find(|theme| theme.label() == label) {
                settings.theme = theme;
            }
        }));

        let font = choice(
            FontChoice::ALL.map(FontChoice::label).as_slice(),
            settings.font.label(),
            window,
            cx,
        );
        subscriptions.push(on_choice(&font, window, cx, |label, settings| {
            if let Some(font) = FontChoice::ALL.into_iter().find(|font| font.label() == label) {
                settings.font = font;
            }
        }));

        let precisions: Vec<String> = (0..=MAX_PRECISION).map(|digits| digits.to_string()).collect();
        let precision_labels: Vec<&str> = precisions.iter().map(String::as_str).collect();
        let precision = choice(
            &precision_labels,
            &settings.engine.precision.to_string(),
            window,
            cx,
        );
        subscriptions.push(on_choice(&precision, window, cx, |label, settings| {
            if let Ok(digits) = label.parse::<u8>() {
                settings.engine.precision = digits.min(MAX_PRECISION);
            }
        }));

        let number_style = choice(
            NUMBER_STYLES.map(|(_, label)| label).as_slice(),
            label_of(&NUMBER_STYLES, settings.engine.number_style),
            window,
            cx,
        );
        subscriptions.push(on_choice(&number_style, window, cx, |label, settings| {
            if let Some(style) = value_of(&NUMBER_STYLES, label) {
                settings.engine.number_style = style;
            }
        }));

        let angle_unit = choice(
            ANGLE_UNITS.map(|(_, label)| label).as_slice(),
            label_of(&ANGLE_UNITS, settings.engine.angle_unit),
            window,
            cx,
        );
        subscriptions.push(on_choice(&angle_unit, window, cx, |label, settings| {
            if let Some(unit) = value_of(&ANGLE_UNITS, label) {
                settings.engine.angle_unit = unit;
            }
        }));

        let time_format = choice(
            TIME_FORMATS.map(|(_, label)| label).as_slice(),
            label_of(&TIME_FORMATS, settings.engine.time_format),
            window,
            cx,
        );
        subscriptions.push(on_choice(&time_format, window, cx, |label, settings| {
            if let Some(format) = value_of(&TIME_FORMATS, label) {
                settings.engine.time_format = format;
            }
        }));

        Self {
            theme,
            font,
            precision,
            number_style,
            angle_unit,
            time_format,
            recording_hotkey: None,
            _subscriptions: subscriptions,
        }
    }

    fn record_hotkey(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let target = window.window_handle();
        let view = cx.entity().downgrade();
        self.recording_hotkey = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle() != target {
                return;
            }
            cx.stop_propagation();
            let keystroke = &event.keystroke;
            let shortcut = hotkey::from_keystroke(keystroke);
            if shortcut.is_none() && keystroke.key != "escape" {
                return; // keep listening until a usable combination arrives
            }
            let _ = view.update(cx, |this, cx| {
                this.recording_hotkey = None;
                cx.notify();
            });
            if let Some(shortcut) = shortcut {
                app::update_settings(cx, |settings| settings.hotkey = Some(shortcut));
            }
        }));
        cx.notify();
    }

    fn choose_notes_folder(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Use as Notes Folder".into()),
        });
        cx.spawn_in(window, async move |_, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            if let Some(dir) = paths.into_iter().next() {
                let _ =
                    cx.update(|_, cx| app::update_settings(cx, |settings| settings.notes_dir = Some(dir)));
            }
        })
        .detach();
    }
}

/// A select over fixed labels with `selected` chosen.
fn choice(
    labels: &[&str],
    selected: &str,
    window: &mut Window,
    cx: &mut Context<'_, SettingsWindow>,
) -> Choice {
    let items: Vec<SharedString> = labels
        .iter()
        .map(|label| SharedString::from((*label).to_owned()))
        .collect();
    let index = labels
        .iter()
        .position(|label| *label == selected)
        .map(IndexPath::new);
    cx.new(|cx| SelectState::new(SearchableVec::new(items), index, window, cx))
}

/// Applies `apply` to the settings when the user picks a label.
fn on_choice(
    choice: &Choice,
    window: &mut Window,
    cx: &mut Context<'_, SettingsWindow>,
    apply: impl Fn(&str, &mut AppSettings) + 'static,
) -> Subscription {
    cx.subscribe_in(
        choice,
        window,
        move |_, _, event: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
            let SelectEvent::Confirm(Some(label)) = event else {
                return;
            };
            let label = label.to_string();
            app::update_settings(cx, |settings| apply(&label, settings));
        },
    )
}

fn label_of<T: PartialEq + Copy>(table: &[(T, &'static str)], value: T) -> &'static str {
    table
        .iter()
        .find(|(candidate, _)| *candidate == value)
        .map_or("", |(_, label)| label)
}

fn value_of<T: Copy>(table: &[(T, &'static str)], label: &str) -> Option<T> {
    table
        .iter()
        .find(|(_, candidate)| *candidate == label)
        .map(|(value, _)| *value)
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let napkin = cx.global::<Napkin>();
        let palette = napkin.palette();
        let settings = napkin.settings.clone();
        let notes_dir = napkin.notes_dir().display().to_string();
        let extensions = napkin.extensions.clone();
        let problems = napkin.problems.clone();
        let extension_dir = napkin.paths.extension_dir().display().to_string();
        let recording = self.recording_hotkey.is_some();

        let section = |title: &'static str| {
            div()
                .pt(px(18.))
                .pb(px(6.))
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.muted)
                .child(title.to_uppercase())
        };
        let row = |label: &'static str, control: AnyElement| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(16.))
                .py(px(6.))
                .child(div().text_size(px(13.)).child(label))
                .child(div().flex_none().w(px(230.)).flex().justify_end().child(control))
        };
        let select = |state: &Choice| Select::new(state).small().w(px(230.)).into_any_element();
        let switch = |id: &'static str, checked: bool, apply: fn(&mut AppSettings, bool)| {
            Switch::new(id)
                .checked(checked)
                .on_click(move |checked: &bool, _, cx| {
                    let checked = *checked;
                    app::update_settings(cx, |settings| apply(settings, checked));
                })
                .into_any_element()
        };

        let font_size =
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(Button::new("font-smaller").small().label("−").on_click(
                    |_: &ClickEvent, window, cx| {
                        window.dispatch_action(Box::new(DecreaseFontSize), cx);
                    },
                ))
                .child(
                    div()
                        .w(px(44.))
                        .text_center()
                        .text_size(px(13.))
                        .child(format!("{} pt", settings.clamped_font_size())),
                )
                .child(Button::new("font-larger").small().label("+").on_click(
                    |_: &ClickEvent, window, cx| {
                        window.dispatch_action(Box::new(IncreaseFontSize), cx);
                    },
                ))
                .into_any_element();

        let hotkey_label = if recording {
            "Press keys…".to_owned()
        } else {
            settings
                .hotkey
                .as_deref()
                .map_or_else(|| "Off".to_owned(), hotkey::display)
        };
        let hotkey_control = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(
                Button::new("hotkey")
                    .small()
                    .label(hotkey_label)
                    .selected(recording)
                    .tooltip("Click, then press the new shortcut (esc cancels)")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.record_hotkey(window, cx))),
            )
            .child(
                Button::new("hotkey-off")
                    .small()
                    .ghost()
                    .label(if settings.hotkey.is_some() {
                        "Turn off"
                    } else {
                        "Default"
                    })
                    .on_click(|_: &ClickEvent, _, cx| {
                        app::update_settings(cx, |settings| {
                            settings.hotkey = if settings.hotkey.is_some() {
                                None
                            } else {
                                Some(DEFAULT_HOTKEY.to_owned())
                            };
                        });
                    }),
            )
            .into_any_element();

        let notes_folder = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .py(px(6.))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(palette.muted)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(notes_dir),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .child(Button::new("notes-choose").small().label("Choose…").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.choose_notes_folder(window, cx)),
                    ))
                    .child(
                        Button::new("notes-reveal")
                            .small()
                            .ghost()
                            .label("Show in Finder")
                            .on_click(|_: &ClickEvent, window, cx| {
                                window.dispatch_action(Box::new(crate::actions::OpenNotesFolder), cx);
                            }),
                    )
                    .when(settings.notes_dir.is_some(), |buttons| {
                        buttons.child(
                            Button::new("notes-default")
                                .small()
                                .ghost()
                                .label("Use default")
                                .on_click(|_: &ClickEvent, _, cx| {
                                    app::update_settings(cx, |settings| settings.notes_dir = None)
                                }),
                        )
                    }),
            );

        let extension_status = if extensions.loaded.is_empty() && extensions.failed.is_empty() {
            "No extensions loaded".to_owned()
        } else {
            format!(
                "{} loaded, {} failed",
                extensions.loaded.len(),
                extensions.failed.len()
            )
        };

        div()
            .id("settings")
            .size_full()
            .overflow_y_scroll()
            .bg(palette.background)
            .text_color(palette.text)
            .px(px(28.))
            .pb(px(28.))
            .child(section("Appearance"))
            .child(row("Theme", select(&self.theme)))
            .child(row("Font", select(&self.font)))
            .child(row("Font size", font_size))
            .child(section("Calculation"))
            .child(row("Precision (decimal places)", select(&self.precision)))
            .child(row("Number style", select(&self.number_style)))
            .child(row("Angle unit", select(&self.angle_unit)))
            .child(row("Time format", select(&self.time_format)))
            .child(section("Notes"))
            .child(notes_folder)
            .child(section("System"))
            .child(row("Show / hide hotkey", hotkey_control))
            .child(row(
                "Menu bar icon",
                switch("menu-bar-icon", settings.desktop.menu_bar_icon, |settings, on| {
                    settings.desktop.menu_bar_icon = on
                }),
            ))
            .child(row(
                "Dock icon",
                switch("dock-icon", settings.desktop.dock_icon, |settings, on| {
                    settings.desktop.dock_icon = on
                }),
            ))
            .child(row(
                "Launch at login",
                switch(
                    "launch-at-login",
                    settings.desktop.launch_at_login,
                    |settings, on| {
                        settings.desktop.launch_at_login = on;
                    },
                ),
            ))
            .child(row(
                "Alfred integration (port 15055)",
                switch("alfred", settings.alfred_http, |settings, on| {
                    settings.alfred_http = on
                }),
            ))
            .child(section("Extensions"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .py(px(6.))
                    .child(div().text_size(px(13.)).child(extension_status))
                    .children(extensions.failed.iter().map(|(file, reason)| {
                        let name = file
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        div()
                            .text_size(px(12.))
                            .text_color(palette.spans.function)
                            .child(format!("{name}: {reason}"))
                    }))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(palette.muted)
                            .child(extension_dir),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.))
                            .child(
                                Button::new("reload-extensions")
                                    .small()
                                    .label("Reload Extensions")
                                    .on_click(|_: &ClickEvent, _, cx| app::reload_extensions(cx)),
                            )
                            .child(
                                Button::new("open-extensions")
                                    .small()
                                    .ghost()
                                    .label("Open Extensions Folder")
                                    .on_click(|_: &ClickEvent, _, cx| app::open_extensions_folder(cx)),
                            ),
                    ),
            )
            .when(!problems.is_empty(), |page| {
                page.child(section("Problems"))
                    .children(problems.into_iter().map(|problem| {
                        div()
                            .text_size(px(12.))
                            .text_color(palette.spans.function)
                            .py(px(2.))
                            .child(problem)
                    }))
            })
    }
}
