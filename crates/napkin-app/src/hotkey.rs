//! The global show/hide hotkey (default ⌥⌘N), registered with the system through
//! `global-hotkey` (Carbon hot keys on macOS, which run on GPUI's main run loop).

use anyhow::Context as _;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use gpui_kit::Keystroke;

/// The registered hotkey.
pub(crate) struct GlobalHotkey {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
}

impl std::fmt::Debug for GlobalHotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalHotkey")
            .field("current", &self.current)
            .finish_non_exhaustive()
    }
}

impl GlobalHotkey {
    /// Installs the hotkey handler; each press sends `()` on `presses`. Call once per process
    /// (the event handler is global).
    pub(crate) fn install(presses: async_channel::Sender<()>) -> anyhow::Result<Self> {
        let manager = GlobalHotKeyManager::new().context("starting the hotkey manager")?;
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                let _ = presses.try_send(());
            }
        }));
        Ok(Self {
            manager,
            current: None,
        })
    }

    /// Replaces the registered hotkey; `None` disables it.
    pub(crate) fn set(&mut self, shortcut: Option<&str>) -> anyhow::Result<()> {
        if let Some(current) = self.current.take() {
            self.manager
                .unregister(current)
                .context("unregistering the old hotkey")?;
        }
        let Some(shortcut) = shortcut else {
            return Ok(());
        };
        let hotkey: HotKey = shortcut
            .parse()
            .with_context(|| format!("reading the hotkey {shortcut:?}"))?;
        self.manager
            .register(hotkey)
            .with_context(|| format!("registering {}", display(shortcut)))?;
        self.current = Some(hotkey);
        Ok(())
    }
}

/// The `global-hotkey` notation (`Alt+Cmd+N`) of a key press, `None` without a modifier that
/// makes it safe to grab globally (⌘, ⌃ or ⌥).
pub(crate) fn from_keystroke(keystroke: &Keystroke) -> Option<String> {
    let modifiers = &keystroke.modifiers;
    if !(modifiers.platform || modifiers.control || modifiers.alt) {
        return None;
    }
    let key = key_token(&keystroke.key)?;
    let mut parts = Vec::new();
    if modifiers.control {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.platform {
        parts.push("Cmd");
    }
    let shortcut = format!("{}+{key}", parts.join("+"));
    shortcut.parse::<HotKey>().is_ok().then_some(shortcut)
}

/// GPUI key names as `global-hotkey` key tokens.
fn key_token(key: &str) -> Option<String> {
    let token = match key {
        "space" => "Space".to_owned(),
        "enter" => "Enter".to_owned(),
        "tab" => "Tab".to_owned(),
        "backspace" => "Backspace".to_owned(),
        "escape" | "shift" | "control" | "alt" | "platform" | "function" | "" => return None,
        key if key.chars().count() == 1 => key.to_uppercase(),
        key if key.starts_with('f') && key[1..].parse::<u8>().is_ok() => key.to_uppercase(),
        _ => return None,
    };
    Some(token)
}

/// A shortcut in Mac notation: `Alt+Cmd+N` becomes `⌥⌘N`.
pub(crate) fn display(shortcut: &str) -> String {
    let mut symbols = String::new();
    let mut key = "";
    for part in shortcut.split('+') {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => symbols.push('⌃'),
            "alt" | "option" => symbols.push('⌥'),
            "shift" => symbols.push('⇧'),
            "cmd" | "command" | "super" => symbols.push('⌘'),
            _ => key = part,
        }
    }
    let key = key
        .strip_prefix("Key")
        .or_else(|| key.strip_prefix("Digit"))
        .unwrap_or(key);
    format!("{symbols}{}", key.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystrokes_become_hotkeys() {
        let keystroke = Keystroke::parse("alt-cmd-n").expect("parsed");
        assert_eq!(from_keystroke(&keystroke).as_deref(), Some("Alt+Cmd+N"));
        let keystroke = Keystroke::parse("ctrl-shift-space").expect("parsed");
        assert_eq!(from_keystroke(&keystroke).as_deref(), Some("Ctrl+Shift+Space"));
    }

    #[test]
    fn plain_keys_are_not_global_hotkeys() {
        assert_eq!(from_keystroke(&Keystroke::parse("n").expect("parsed")), None);
        assert_eq!(
            from_keystroke(&Keystroke::parse("shift-n").expect("parsed")),
            None
        );
    }

    #[test]
    fn the_default_parses_and_displays_like_a_mac_menu() {
        assert!(crate::settings::DEFAULT_HOTKEY.parse::<HotKey>().is_ok());
        assert_eq!(display("Alt+Cmd+N"), "⌥⌘N");
        assert_eq!(display("Ctrl+Shift+Space"), "⌃⇧SPACE");
    }
}
