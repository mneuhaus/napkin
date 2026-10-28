//! User settings (`settings.json`) and window state (`state.json`), both plain JSON.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use napkin_core::InputLanguage;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::theme::ThemeChoice;

/// Default editor font size in points.
pub(crate) const DEFAULT_FONT_SIZE: f32 = 15.;
/// Smallest and largest font size `⌘-` / `⌘+` reach.
pub(crate) const FONT_SIZE_RANGE: (f32, f32) = (10., 36.);
/// Numi's default global hotkey, ⌥⌘N, in `global-hotkey` notation.
pub(crate) const DEFAULT_HOTKEY: &str = "Alt+Cmd+N";

/// The state of an on/off setting, so call sites read `Switch::On` instead of a bare `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Switch {
    On,
    Off,
}

impl Switch {
    pub(crate) fn is_on(self) -> bool {
        self == Self::On
    }
}

impl From<bool> for Switch {
    fn from(on: bool) -> Self {
        if on { Self::On } else { Self::Off }
    }
}

/// The editor typeface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum FontChoice {
    /// The system font, like Numi.
    #[default]
    System,
    /// A monospace font.
    Monospace,
}

impl FontChoice {
    pub(crate) const ALL: [Self; 2] = [Self::System, Self::Monospace];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Monospace => "Monospace",
        }
    }

    /// The GPUI font family name.
    pub(crate) fn family(self) -> &'static str {
        match self {
            Self::System => ".SystemUIFont",
            Self::Monospace => "Menlo",
        }
    }
}

/// How the app shows up on the desktop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Desktop {
    /// Show the menu bar icon that toggles the window.
    pub(crate) menu_bar_icon: bool,
    /// Show the Dock icon (off: the app lives in the menu bar only).
    pub(crate) dock_icon: bool,
    /// Start Napkin when the user logs in.
    pub(crate) launch_at_login: bool,
}

impl Default for Desktop {
    fn default() -> Self {
        Self {
            menu_bar_icon: true,
            dock_icon: true,
            launch_at_login: false,
        }
    }
}

/// Everything the settings window edits, persisted as `settings.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppSettings {
    pub(crate) theme: ThemeChoice,
    pub(crate) font: FontChoice,
    pub(crate) font_size: f32,
    /// Precision, number style, angle unit and time format, handed to the engine.
    pub(crate) engine: napkin_core::Settings,
    /// The input language; `None` is "Automatic" and follows the system language.
    pub(crate) input_language: Option<InputLanguage>,
    /// Notes folder; `None` uses the default inside the support folder.
    pub(crate) notes_dir: Option<PathBuf>,
    /// Global show/hide hotkey (`Alt+Cmd+N`); `None` disables it.
    pub(crate) hotkey: Option<String>,
    pub(crate) desktop: Desktop,
    /// Answer `http://127.0.0.1:15055/?q=` like Numi's Alfred integration.
    pub(crate) alfred_http: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::default(),
            font: FontChoice::default(),
            font_size: DEFAULT_FONT_SIZE,
            engine: napkin_core::Settings::default(),
            input_language: None,
            notes_dir: None,
            hotkey: Some(DEFAULT_HOTKEY.to_owned()),
            desktop: Desktop::default(),
            alfred_http: true,
        }
    }
}

impl AppSettings {
    /// The settings handed to the engine, with "Automatic" resolved to the system language.
    pub(crate) fn engine_settings(&self) -> napkin_core::Settings {
        napkin_core::Settings {
            input_language: self
                .input_language
                .unwrap_or_else(crate::platform::system_language),
            ..self.engine.clone()
        }
    }

    /// The font size clamped to the supported range.
    pub(crate) fn clamped_font_size(&self) -> f32 {
        let (min, max) = FONT_SIZE_RANGE;
        if self.font_size.is_finite() {
            self.font_size.clamp(min, max)
        } else {
            DEFAULT_FONT_SIZE
        }
    }
}

/// A window's frame in screen points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct WindowFrame {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Remembered between launches, persisted as `state.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct WindowState {
    pub(crate) frame: Option<WindowFrame>,
    /// File name of the note that was open.
    pub(crate) last_note: Option<String>,
}

/// Reads a JSON file, falling back to the default when it is missing or unreadable.
pub(crate) fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    match fs::read_to_string(path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_else(|error| {
            eprintln!("ignoring {}: {error}", path.display());
            T::default()
        }),
        Err(_) => T::default(),
    }
}

/// Writes a JSON file atomically (temporary file, then rename).
pub(crate) fn save_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(value).context("serialising")?;
    write_atomically(path, json.as_bytes())
}

/// Replaces `path` with `contents` without leaving a half-written file behind.
pub(crate) fn write_atomically(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("napkin");
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    fs::write(&temporary, contents).with_context(|| format!("writing {}", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| format!("replacing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/test-settings")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch folder");
        dir
    }

    #[test]
    fn settings_round_trip_through_json() {
        let dir = scratch("round-trip");
        let path = dir.join("settings.json");
        let mut settings = AppSettings {
            font_size: 18.,
            hotkey: None,
            ..AppSettings::default()
        };
        settings.engine.precision = 4;
        settings.desktop.dock_icon = false;
        save_json(&path, &settings).expect("saved");
        assert_eq!(load_json::<AppSettings>(&path), settings);
    }

    #[test]
    fn missing_fields_take_their_defaults() {
        let dir = scratch("partial");
        let path = dir.join("settings.json");
        fs::write(&path, r#"{ "font_size": 20, "desktop": { "dock_icon": false } }"#).expect("written");
        let settings: AppSettings = load_json(&path);
        assert!((settings.font_size - 20.).abs() < f32::EPSILON);
        assert!(!settings.desktop.dock_icon);
        assert!(settings.desktop.menu_bar_icon);
        assert_eq!(settings.hotkey.as_deref(), Some(DEFAULT_HOTKEY));
    }

    #[test]
    fn broken_json_falls_back_to_defaults() {
        let dir = scratch("broken");
        let path = dir.join("settings.json");
        fs::write(&path, "{ nope").expect("written");
        assert_eq!(load_json::<AppSettings>(&path), AppSettings::default());
    }

    #[test]
    fn font_size_is_clamped() {
        let settings = AppSettings {
            font_size: 99.,
            ..AppSettings::default()
        };
        assert!((settings.clamped_font_size() - FONT_SIZE_RANGE.1).abs() < f32::EPSILON);
    }
}
