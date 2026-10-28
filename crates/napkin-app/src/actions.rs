//! Actions, Numi's key bindings and the menu bar.

use gpui_kit::{App, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType};

gpui_kit::actions!(
    napkin,
    [
        Quit,
        HideApp,
        HideOthers,
        ShowAll,
        CloseWindow,
        Minimize,
        ShowSettings,
        ToggleWindow,
        NewNote,
        ToggleNotesList,
        NextNote,
        PreviousNote,
        Import,
        Export,
        Print,
        SurroundWithParentheses,
        CopyLineResult,
        DeleteAll,
        CopyAll,
        IncreaseFontSize,
        DecreaseFontSize,
        ResetFontSize,
        ReloadExtensions,
        OpenExtensionsFolder,
        OpenNotesFolder,
    ]
);

/// Binds Numi's shortcuts. Several keys are bound in both spellings GPUI may report for a shifted
/// key (`cmd-shift-]` and `cmd-}`), so they work whatever the keyboard layout reports.
pub(crate) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("cmd-,", ShowSettings, None),
        KeyBinding::new("cmd-n", NewNote, None),
        KeyBinding::new("cmd-l", ToggleNotesList, None),
        KeyBinding::new("cmd-shift-]", NextNote, None),
        KeyBinding::new("cmd-}", NextNote, None),
        KeyBinding::new("cmd-shift-[", PreviousNote, None),
        KeyBinding::new("cmd-{", PreviousNote, None),
        KeyBinding::new("cmd-o", Import, None),
        KeyBinding::new("cmd-s", Export, None),
        KeyBinding::new("cmd-p", Print, None),
        KeyBinding::new("cmd-shift-0", SurroundWithParentheses, None),
        KeyBinding::new("cmd-)", SurroundWithParentheses, None),
        KeyBinding::new("cmd-shift-c", CopyLineResult, None),
        KeyBinding::new("alt-cmd-backspace", DeleteAll, None),
        KeyBinding::new("alt-cmd-c", CopyAll, None),
        KeyBinding::new("cmd-=", IncreaseFontSize, None),
        KeyBinding::new("cmd-+", IncreaseFontSize, None),
        KeyBinding::new("cmd-shift-=", IncreaseFontSize, None),
        KeyBinding::new("cmd--", DecreaseFontSize, None),
        KeyBinding::new("cmd-0", ResetFontSize, None),
    ]);
}

/// The menu bar. Edit items dispatch the editor's own actions through the native menu, so the
/// system's undo, cut, copy and paste behave as in any Mac app.
pub(crate) fn set_menus(cx: &mut App) {
    use gpui_kit::base::input;

    cx.set_menus([
        Menu::new("Napkin").items([
            MenuItem::action("Settings…", ShowSettings),
            MenuItem::separator(),
            MenuItem::action("Reload Extensions", ReloadExtensions),
            MenuItem::action("Open Extensions Folder", OpenExtensionsFolder),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide Napkin", HideApp),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::action("Show All", ShowAll),
            MenuItem::separator(),
            MenuItem::action("Quit Napkin", Quit),
        ]),
        Menu::new("File").items([
            MenuItem::action("New Note", NewNote),
            MenuItem::action("Notes…", ToggleNotesList),
            MenuItem::separator(),
            MenuItem::action("Next Note", NextNote),
            MenuItem::action("Previous Note", PreviousNote),
            MenuItem::separator(),
            MenuItem::action("Import…", Import),
            MenuItem::action("Export…", Export),
            MenuItem::action("Open Notes Folder", OpenNotesFolder),
            MenuItem::separator(),
            MenuItem::action("Print…", Print),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow),
        ]),
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", input::Undo, OsAction::Undo),
            MenuItem::os_action("Redo", input::Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", input::Cut, OsAction::Cut),
            MenuItem::os_action("Copy", input::Copy, OsAction::Copy),
            MenuItem::os_action("Paste", input::Paste, OsAction::Paste),
            MenuItem::os_action("Select All", input::SelectAll, OsAction::SelectAll),
            MenuItem::separator(),
            MenuItem::action("Copy Line Result", CopyLineResult),
            MenuItem::action("Copy All", CopyAll),
            MenuItem::action("Surround with Parentheses", SurroundWithParentheses),
            MenuItem::action("Delete All", DeleteAll),
        ]),
        Menu::new("View").items([
            MenuItem::action("Increase Font Size", IncreaseFontSize),
            MenuItem::action("Decrease Font Size", DecreaseFontSize),
            MenuItem::action("Reset Font Size", ResetFontSize),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", Minimize),
            MenuItem::action("Show Napkin", ToggleWindow),
        ]),
    ]);
}
