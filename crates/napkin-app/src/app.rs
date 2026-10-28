//! The application: shared state, settings changes, background services (rates, menu bar icon,
//! global hotkey, Alfred endpoint, login item) and the app-level actions.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, AsyncApp, Bounds, Context, Entity, Global, Pixels, Window,
    WindowAppearance, WindowBounds, point, px, size,
};
use napkin_core::{Engine, ExchangeRates};
use napkin_extensions::LoadReport;

use crate::actions::{
    CloseWindow, CopyAll, CopyLineResult, DecreaseFontSize, DeleteAll, Export, HideApp, HideOthers, Import,
    IncreaseFontSize, Minimize, NewNote, NextNote, OpenExtensionsFolder, OpenNotesFolder, PreviousNote,
    Print, Quit, ReloadExtensions, ResetFontSize, ShowAll, ShowSettings, SurroundWithParentheses,
    ToggleNotesList, ToggleWindow,
};
use crate::editor::EditorFont;
use crate::hotkey::GlobalHotkey;
use crate::http::{self, AlfredServer};
use crate::note_window::NoteWindow;
use crate::notes::NoteStore;
use crate::paths::AppPaths;
use crate::settings::{
    AppSettings, DEFAULT_FONT_SIZE, FONT_SIZE_RANGE, WindowFrame, WindowState, load_json, save_json,
};

use crate::sheet::SharedEngine;
use crate::theme::Palette;
use crate::tray::{MenuBarIcon, TrayCommand};
use crate::{login_item, platform};

/// Default window size, as in the spec.
const DEFAULT_WINDOW: (f32, f32) = (560., 440.);
/// How often the app checks whether the cached rates went stale.
const RATE_CHECK_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// Font size step of `⌘+` / `⌘-`.
const FONT_STEP: f32 = 1.;

/// How the app was started.
#[derive(Debug, Clone, Default)]
pub(crate) struct Launch {
    /// A `.numi` / `.txt` file to open as a new note.
    pub(crate) open_file: Option<PathBuf>,
    /// Testing hook (`NAPKIN_BACKGROUND=1`): open without focus and never activate the app.
    pub(crate) background: bool,
}

/// The main window and its view (the window itself holds gpui-kit's `Root`).
#[derive(Clone)]
pub(crate) struct MainWindow {
    window: AnyWindowHandle,
    view: Entity<NoteWindow>,
}

impl MainWindow {
    fn update<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(&mut NoteWindow, &mut Window, &mut Context<'_, NoteWindow>) -> R,
    ) -> anyhow::Result<R> {
        let view = self.view.clone();
        self.window.update(cx, |_, window, cx| {
            view.update(cx, |view, cx| f(view, window, cx))
        })
    }
}

/// State shared by every window.
pub(crate) struct Napkin {
    pub(crate) paths: AppPaths,
    pub(crate) settings: AppSettings,
    pub(crate) engine: SharedEngine,
    pub(crate) extensions: LoadReport,
    /// The system appearance, for the System theme.
    pub(crate) appearance: WindowAppearance,
    /// Problems worth showing in settings (a hotkey that could not be registered, ...).
    pub(crate) problems: Vec<String>,
    pub(crate) background: bool,
    main_window: Option<MainWindow>,
    settings_window: Option<AnyWindowHandle>,
    tray: Option<MenuBarIcon>,
    hotkey: Option<GlobalHotkey>,
    alfred: Option<AlfredServer>,
}

impl Global for Napkin {}

impl Napkin {
    pub(crate) fn palette(&self) -> Palette {
        self.settings.theme.palette(self.appearance)
    }

    pub(crate) fn editor_font(&self) -> EditorFont {
        EditorFont {
            choice: self.settings.font,
            size: self.settings.clamped_font_size(),
        }
    }

    pub(crate) fn notes_dir(&self) -> PathBuf {
        self.settings
            .notes_dir
            .clone()
            .unwrap_or_else(|| self.paths.default_notes_dir())
    }

    pub(crate) fn note_store(&self) -> NoteStore {
        NoteStore::new(self.notes_dir())
    }

    pub(crate) fn window_state(&self) -> WindowState {
        load_json(&self.paths.state_file())
    }

    pub(crate) fn save_window_state(&self, state: &WindowState) {
        if let Err(error) = save_json(&self.paths.state_file(), state) {
            eprintln!("could not save the window state: {error:#}");
        }
    }
}

/// Starts the app: state, services, actions, the main window.
pub(crate) fn start(
    launch: Launch,
    opened_files: async_channel::Receiver<Vec<String>>,
    cx: &mut App,
) -> anyhow::Result<()> {
    let paths = AppPaths::resolve()?;
    let settings: AppSettings = load_json(&paths.settings_file());
    let rates = paths.rate_cache().load_or_bundled();
    let (engine, extensions) = build_engine(&settings.engine, rates, &paths);
    for (file, reason) in &extensions.failed {
        eprintln!("extension {} failed: {reason}", file.display());
    }
    cx.set_global(Napkin {
        paths,
        settings,
        engine: Arc::new(RwLock::new(engine)),
        extensions,
        appearance: cx.window_appearance(),
        problems: Vec::new(),
        background: launch.background,
        main_window: None,
        settings_window: None,
        tray: None,
        hotkey: None,
        alfred: None,
    });

    crate::actions::bind_keys(cx);
    crate::actions::set_menus(cx);
    register_actions(cx);
    apply_palette(cx);

    let settings = cx.global::<Napkin>().settings.clone();
    platform::set_dock_icon_visible(settings.desktop.dock_icon);
    install_menu_bar_icon(settings.desktop.menu_bar_icon, cx);
    install_hotkey(settings.hotkey.as_deref(), cx);
    set_alfred_endpoint(settings.alfred_http, cx);
    apply_login_item(settings.desktop.launch_at_login, cx);

    open_main_window(launch.open_file, cx)?;
    import_opened_files(opened_files, cx);
    refresh_rates_when_stale(cx);
    Ok(())
}

/// Imports files opened from Finder (`file://` URLs) as new notes.
fn import_opened_files(opened_files: async_channel::Receiver<Vec<String>>, cx: &mut App) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        while let Ok(urls) = opened_files.recv().await {
            for path in urls.iter().filter_map(|url| file_url_path(url)) {
                let _ = cx.update(|cx| {
                    with_main_window(cx, move |view, window, cx| view.import_file(&path, window, cx));
                });
            }
        }
    })
    .detach();
}

/// The local path of a `file://` URL, percent-decoded.
fn file_url_path(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix("file://")?;
    let path = path.strip_prefix("localhost").unwrap_or(path);
    Some(PathBuf::from(crate::http::percent_decode_path(path)))
}

/// A fresh engine with the given settings, rates and every extension loaded.
fn build_engine(
    settings: &napkin_core::Settings,
    rates: ExchangeRates,
    paths: &AppPaths,
) -> (Engine, LoadReport) {
    let mut engine = Engine::with_settings(settings.clone());
    engine.set_exchange_rates(rates);
    let report = napkin_extensions::load_into(&mut engine, &paths.extension_dirs());
    (engine, report)
}

fn register_actions(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &HideApp, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &ShowSettings, cx| show_settings(cx));
    cx.on_action(|_: &ToggleWindow, cx| toggle_main_window(cx));
    cx.on_action(|_: &CloseWindow, cx| close_key_window(cx));
    cx.on_action(|_: &Minimize, cx| {
        if let Some(window) = cx.active_window() {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.minimize_window());
            });
        }
    });
    cx.on_action(|_: &IncreaseFontSize, cx| change_font_size(cx, |size| size + FONT_STEP));
    cx.on_action(|_: &DecreaseFontSize, cx| change_font_size(cx, |size| size - FONT_STEP));
    cx.on_action(|_: &ResetFontSize, cx| change_font_size(cx, |_| DEFAULT_FONT_SIZE));
    cx.on_action(|_: &ReloadExtensions, cx| reload_extensions(cx));
    cx.on_action(|_: &OpenExtensionsFolder, cx| open_extensions_folder(cx));
    cx.on_action(|_: &OpenNotesFolder, cx| {
        let dir = cx.global::<Napkin>().notes_dir();
        let _ = std::fs::create_dir_all(&dir);
        cx.open_with_system(&dir);
    });

    // Note actions reach the main window from anywhere: a menu, the settings window, a key.
    cx.on_action(|_: &NewNote, cx| with_main_window(cx, NoteWindow::new_note));
    cx.on_action(|_: &ToggleNotesList, cx| with_main_window(cx, NoteWindow::toggle_notes_list));
    cx.on_action(|_: &NextNote, cx| with_main_window(cx, |this, window, cx| this.step_note(1, window, cx)));
    cx.on_action(|_: &PreviousNote, cx| {
        with_main_window(cx, |this, window, cx| this.step_note(-1, window, cx))
    });
    cx.on_action(|_: &Import, cx| with_main_window(cx, NoteWindow::import));
    cx.on_action(|_: &Export, cx| with_main_window(cx, NoteWindow::export));
    cx.on_action(|_: &Print, cx| with_main_window(cx, NoteWindow::print));
    cx.on_action(|_: &SurroundWithParentheses, cx| {
        with_main_window(cx, NoteWindow::surround_with_parentheses)
    });
    cx.on_action(|_: &CopyLineResult, cx| with_main_window(cx, NoteWindow::copy_current_line_result));
    cx.on_action(|_: &DeleteAll, cx| with_main_window(cx, NoteWindow::delete_all));
    cx.on_action(|_: &CopyAll, cx| with_main_window(cx, NoteWindow::copy_all));
}

/// Runs `f` on the main window after the current dispatch (the window may be mid-update now),
/// showing the window first.
pub(crate) fn with_main_window(
    cx: &mut App,
    f: impl FnOnce(&mut NoteWindow, &mut Window, &mut Context<'_, NoteWindow>) + 'static,
) {
    cx.defer(move |cx| {
        show_main_window(cx);
        let Some(main) = cx.global::<Napkin>().main_window.clone() else {
            return;
        };
        let _ = main.update(cx, f);
    });
}

/// Opens the main window, restoring its last frame, with `open_file` imported as a new note.
fn open_main_window(open_file: Option<PathBuf>, cx: &mut App) -> anyhow::Result<()> {
    let napkin = cx.global::<Napkin>();
    let background = napkin.background;
    let state = napkin.window_state();
    let bounds = restored_bounds(state.frame, cx);
    let options = gpui_kit::WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("Napkin".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(13.), px(13.))),
        }),
        window_min_size: Some(size(px(300.), px(180.))),
        focus: !background,
        app_id: Some("com.neuhaus.napkin".into()),
        ..Default::default()
    };
    let (window_handle, view) = gpui_kit::open_window(options, cx, |window, cx| {
        let view = cx.new(|cx| NoteWindow::new(state.last_note.as_deref(), window, cx));
        window.on_window_should_close(cx, |_, cx| {
            // Closing hides Napkin, like Numi: it keeps running in the menu bar and on the hotkey.
            cx.defer(|cx| cx.hide());
            false
        });
        view.update(cx, |view, cx| view.focus_editor(window, cx));
        view
    })?;
    cx.global_mut::<Napkin>().main_window = Some(MainWindow {
        window: window_handle,
        view,
    });
    if let Some(path) = open_file {
        with_main_window(cx, move |view, window, cx| view.import_file(&path, window, cx));
    }
    if !background {
        cx.activate(true);
    }
    Ok(())
}

/// The saved frame when it is still on a screen, else the default size centred.
fn restored_bounds(frame: Option<WindowFrame>, cx: &App) -> Bounds<Pixels> {
    if let Some(frame) = frame {
        let bounds = Bounds::new(
            point(px(frame.x), px(frame.y)),
            size(px(frame.width), px(frame.height)),
        );
        let visible = cx
            .displays()
            .iter()
            .any(|display| display.bounds().intersects(&bounds));
        if visible && frame.width >= 200. && frame.height >= 120. {
            return bounds;
        }
    }
    Bounds::centered(None, size(px(DEFAULT_WINDOW.0), px(DEFAULT_WINDOW.1)), cx)
}

/// Brings the main window to the front.
pub(crate) fn show_main_window(cx: &mut App) {
    let napkin = cx.global::<Napkin>();
    let (Some(main), background) = (napkin.main_window.clone(), napkin.background) else {
        return;
    };
    if !background {
        cx.activate(true);
    }
    let _ = main.update(cx, move |view, window, cx| {
        if !background {
            window.activate_window();
        }
        view.focus_editor(window, cx);
    });
}

/// Shows the window, or hides Napkin when its window is already in front (menu bar icon, hotkey).
fn toggle_main_window(cx: &mut App) {
    let Some(main) = cx.global::<Napkin>().main_window.clone() else {
        return;
    };
    cx.defer(move |cx| {
        let in_front = main
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap_or(false);
        if in_front { cx.hide() } else { show_main_window(cx) }
    });
}

/// `⌘W`: closes the settings window, hides Napkin for the main window.
fn close_key_window(cx: &mut App) {
    let napkin = cx.global::<Napkin>();
    let settings = napkin.settings_window.map(|handle| handle.window_id());
    let active = cx.active_window().map(|window| window.window_id());
    if active.is_some() && active == settings {
        if let Some(window) = cx.active_window() {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            });
        }
        cx.global_mut::<Napkin>().settings_window = None;
    } else {
        cx.hide();
    }
}

fn show_settings(cx: &mut App) {
    cx.defer(|cx| {
        if let Some(handle) = cx.global::<Napkin>().settings_window
            && handle.update(cx, |_, window, _| window.activate_window()).is_ok()
        {
            cx.activate(true);
            return;
        }
        match crate::settings_window::open(cx) {
            Ok(handle) => cx.global_mut::<Napkin>().settings_window = Some(handle),
            Err(error) => eprintln!("could not open settings: {error:#}"),
        }
        cx.activate(true);
    });
}

/// Called when the settings window closes itself.
pub(crate) fn forget_settings_window(cx: &mut App) {
    cx.global_mut::<Napkin>().settings_window = None;
}

/// Changes settings, saves them, and applies what changed.
pub(crate) fn update_settings(cx: &mut App, change: impl FnOnce(&mut AppSettings)) {
    let before = cx.global::<Napkin>().settings.clone();
    let mut after = before.clone();
    change(&mut after);
    if after == before {
        return;
    }
    let napkin = cx.global_mut::<Napkin>();
    napkin.settings = after.clone();
    if let Err(error) = save_json(&napkin.paths.settings_file(), &after) {
        eprintln!("could not save settings: {error:#}");
    }

    if after.engine != before.engine {
        napkin
            .engine
            .write()
            .expect("engine lock poisoned")
            .set_settings(after.engine.clone());
        reevaluate(cx);
    }
    if after.theme != before.theme || after.font != before.font || after.font_size != before.font_size {
        apply_palette(cx);
    }
    if after.hotkey != before.hotkey {
        install_hotkey(after.hotkey.as_deref(), cx);
    }
    if after.desktop.menu_bar_icon != before.desktop.menu_bar_icon {
        install_menu_bar_icon(after.desktop.menu_bar_icon, cx);
    }
    if after.desktop.dock_icon != before.desktop.dock_icon {
        platform::set_dock_icon_visible(after.desktop.dock_icon);
        // Switching the activation policy can send the app to the back.
        cx.activate(true);
    }
    if after.desktop.launch_at_login != before.desktop.launch_at_login {
        apply_login_item(after.desktop.launch_at_login, cx);
    }
    if after.alfred_http != before.alfred_http {
        set_alfred_endpoint(after.alfred_http, cx);
    }
    if after.notes_dir != before.notes_dir {
        with_main_window(cx, NoteWindow::reload_notes);
    }
    cx.refresh_windows();
}

/// Re-evaluates the open note (after settings, rates or extensions changed).
fn reevaluate(cx: &mut App) {
    let Some(main) = cx.global::<Napkin>().main_window.clone() else {
        return;
    };
    cx.defer(move |cx| {
        let _ = main.update(cx, |view, _, cx| view.reevaluate(cx));
    });
}

/// Applies the theme to gpui-component (settings window, lists, inputs) and repaints.
pub(crate) fn apply_palette(cx: &mut App) {
    let palette = cx.global::<Napkin>().palette();
    palette.apply_to_components(cx);
    cx.refresh_windows();
}

/// Records a system appearance change (for the System theme).
pub(crate) fn set_appearance(appearance: WindowAppearance, cx: &mut App) {
    if cx.global::<Napkin>().appearance != appearance {
        cx.global_mut::<Napkin>().appearance = appearance;
        apply_palette(cx);
    }
}

fn change_font_size(cx: &mut App, change: impl FnOnce(f32) -> f32) {
    let (min, max) = FONT_SIZE_RANGE;
    let current = cx.global::<Napkin>().settings.clamped_font_size();
    let next = change(current).clamp(min, max);
    update_settings(cx, |settings| settings.font_size = next);
}

/// Rebuilds the engine with every extension loaded again (units and functions cannot be
/// unregistered, so a fresh engine is the only clean reload).
pub(crate) fn reload_extensions(cx: &mut App) {
    let napkin = cx.global::<Napkin>();
    let rates = napkin
        .engine
        .read()
        .expect("engine lock poisoned")
        .exchange_rates()
        .clone();
    let (engine, report) = build_engine(&napkin.settings.engine, rates, &napkin.paths);
    let napkin = cx.global_mut::<Napkin>();
    *napkin.engine.write().expect("engine lock poisoned") = engine;
    napkin.extensions = report;
    reevaluate(cx);
    cx.refresh_windows();
}

pub(crate) fn open_extensions_folder(cx: &mut App) {
    let dir = cx.global::<Napkin>().paths.extension_dir();
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("could not create {}: {error}", dir.display());
    }
    cx.open_with_system(&dir);
}

/// Installs the menu bar icon once, then only toggles its visibility.
fn install_menu_bar_icon(visible: bool, cx: &mut App) {
    if let Some(tray) = &cx.global::<Napkin>().tray {
        tray.set_visible(visible);
        return;
    }
    if !visible {
        return;
    }
    let (sender, receiver) = async_channel::unbounded();
    match MenuBarIcon::install(sender) {
        Ok(tray) => cx.global_mut::<Napkin>().tray = Some(tray),
        Err(error) => {
            report_problem(format!("Menu bar icon: {error:#}"), cx);
            return;
        }
    }
    cx.spawn(async move |cx: &mut AsyncApp| {
        while let Ok(command) = receiver.recv().await {
            let _ = cx.update(|cx| match command {
                TrayCommand::ToggleWindow => toggle_main_window(cx),
                TrayCommand::NewNote => with_main_window(cx, NoteWindow::new_note),
                TrayCommand::ShowSettings => show_settings(cx),
                TrayCommand::Quit => cx.quit(),
            });
        }
    })
    .detach();
}

/// Registers the global hotkey (installing the handler on first use).
fn install_hotkey(shortcut: Option<&str>, cx: &mut App) {
    cx.global_mut::<Napkin>()
        .problems
        .retain(|problem| !problem.starts_with("Hotkey"));
    if cx.global::<Napkin>().hotkey.is_none() {
        if shortcut.is_none() {
            return;
        }
        let (sender, receiver) = async_channel::unbounded();
        match GlobalHotkey::install(sender) {
            Ok(hotkey) => cx.global_mut::<Napkin>().hotkey = Some(hotkey),
            Err(error) => {
                report_problem(format!("Hotkey: {error:#}"), cx);
                return;
            }
        }
        cx.spawn(async move |cx: &mut AsyncApp| {
            while receiver.recv().await.is_ok() {
                let _ = cx.update(toggle_main_window);
            }
        })
        .detach();
    }
    let result = cx
        .global_mut::<Napkin>()
        .hotkey
        .as_mut()
        .map(|hotkey| hotkey.set(shortcut));
    if let Some(Err(error)) = result {
        report_problem(format!("Hotkey: {error:#}"), cx);
    }
}

fn set_alfred_endpoint(enabled: bool, cx: &mut App) {
    cx.global_mut::<Napkin>()
        .problems
        .retain(|problem| !problem.starts_with("Alfred"));
    if !enabled {
        cx.global_mut::<Napkin>().alfred = None;
        return;
    }
    if cx.global::<Napkin>().alfred.is_some() {
        return;
    }
    let engine = Arc::clone(&cx.global::<Napkin>().engine);
    match AlfredServer::start(engine, http::PORT) {
        Ok(server) => cx.global_mut::<Napkin>().alfred = Some(server),
        Err(error) => report_problem(format!("Alfred integration: {error:#}"), cx),
    }
}

fn apply_login_item(enabled: bool, cx: &mut App) {
    cx.global_mut::<Napkin>()
        .problems
        .retain(|problem| !problem.starts_with("Launch at login"));
    let dir = cx.global::<Napkin>().paths.launch_agents_dir().to_path_buf();
    if let Err(error) = login_item::apply(&dir, enabled) {
        report_problem(format!("Launch at login: {error:#}"), cx);
    }
}

fn report_problem(problem: String, cx: &mut App) {
    eprintln!("{problem}");
    cx.global_mut::<Napkin>().problems.push(problem);
}

/// Downloads rates in the background when the cache is older than an hour, now and every few
/// minutes while the app runs, and re-evaluates when new rates arrive.
fn refresh_rates_when_stale(cx: &mut App) {
    let cache = cx.global::<Napkin>().paths.rate_cache();
    cx.spawn(async move |cx: &mut AsyncApp| {
        loop {
            if cache.needs_refresh(napkin_rates::DEFAULT_MAX_AGE) {
                let download = cache.clone();
                let refreshed = cx
                    .background_executor()
                    .spawn(async move { download.refresh() })
                    .await;
                match refreshed {
                    Ok(rates) => {
                        let _ = cx.update(|cx| {
                            cx.global::<Napkin>()
                                .engine
                                .write()
                                .expect("engine lock poisoned")
                                .set_exchange_rates(rates);
                            reevaluate(cx);
                        });
                    }
                    Err(error) => eprintln!("rates not refreshed: {error}"),
                }
            }
            cx.background_executor().timer(RATE_CHECK_INTERVAL).await;
        }
    })
    .detach();
}
