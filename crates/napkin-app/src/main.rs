//! Napkin: a calculator notepad. Type math in plain words, read the results on the right.
//!
//! `napkin-app [file.numi]` opens the file as a new note. Environment for testing:
//! `NAPKIN_HOME=<dir>` keeps notes, settings, caches, extensions and the login item in one
//! folder; `NAPKIN_BACKGROUND=1` opens the window without focus and never activates the app.

mod actions;
mod app;
mod editor;
mod hotkey;
mod http;
mod login_item;
mod note_window;
mod notes;
mod notes_list;
mod paths;
mod platform;
mod results;
mod settings;
mod settings_window;
mod sheet;
mod theme;
mod tray;

use std::borrow::Cow;
use std::path::PathBuf;

use gpui_kit::{App, AssetSource, SharedString};

use crate::app::Launch;

gpui_kit::assets::icon_assets!(
    ExtraIconAssets,
    [List, Pencil, Trash, Plus, Search, FolderOpen, X]
);

/// The icons the app uses beyond gpui-component's defaults.
pub(crate) use gpui_kit::assets::IconName as ExtraIcon;

/// gpui-kit's default assets plus [`ExtraIconAssets`].
struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = ExtraIconAssets.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ExtraIconAssets.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

fn launch_from_environment() -> Launch {
    Launch {
        open_file: std::env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .filter(|path| path.is_file()),
        background: std::env::var_os("NAPKIN_BACKGROUND").is_some_and(|value| !value.is_empty()),
    }
}

fn main() {
    let launch = launch_from_environment();
    let application = gpui_kit::application().with_assets(Assets);
    // The Dock icon, or opening Napkin again from Finder, brings the window back.
    application.on_reopen(app::show_main_window);
    // `.numi` files opened from Finder become notes.
    let (opened_sender, opened_files) = async_channel::unbounded();
    application.on_open_urls(move |urls| {
        let _ = opened_sender.try_send(urls);
    });
    application.run(move |cx: &mut App| {
        gpui_kit::init(cx);
        notes_list::bind_keys(cx);
        if let Err(error) = app::start(launch, opened_files, cx) {
            eprintln!("Napkin could not start: {error:#}");
            cx.quit();
        }
    });
}
