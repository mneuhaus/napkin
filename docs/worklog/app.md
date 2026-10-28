# Worklog: napkin-app

Durable log of the desktop app work (branch `app`). Newest milestone first in "Done".

## Done

- **M4: gate.** Lints clean (`cargo clippy -p napkin-app --no-deps --all-targets -- -D warnings`),
  33 unit tests, release bundle builds and launches through Launch Services.
- **M3: bundle + test hooks.** `scripts/bundle-macos.sh` builds `dist/Napkin.app` (bundle id
  `com.neuhaus.napkin`, icon drawn by `scripts/make-icon.swift`, `.numi` document type, ad-hoc
  signed). `NAPKIN_TEST_ACTIONS` dispatches actions after start for unfocused screenshots.
- **M2: app shell.** Global `Napkin` state (paths, settings, shared engine, services), actions,
  Numi key bindings, menu bar, notes store (`.numi` files, newest first), notes list overlay
  (search, switch, rename, delete into the Trash with a native confirmation), import / export /
  print, six themes projected onto gpui-component, settings window (all spec fields, applied
  live), menu bar icon (tray-icon), global hotkey (global-hotkey), launch at login
  (LaunchAgent), Dock icon (activation policy), Alfred endpoint (tiny_http), rates refresh
  loop, extension loading/reload, window frame + last note in `state.json`.
- **M1: editor + result column proven.** gpui-base `EditorState` (code-editor mode, line numbers,
  folding, indent guides, auto-close, smart indent and search off) with a `SheetHighlighter`
  feeding engine spans as highlight names (`napkin.number`, ...) resolved by the theme's
  `HighlightStyleResolver`. `ResultsLayer` (custom element) paints each answer right-aligned on
  the first visual row of its line. Screenshots confirmed wrap and scroll alignment.

## Next (after the engine/rates/extensions merge)

1. Re-check highlighting and results with the real engine (the branch has the stub).
2. `cargo clippy` without `--no-deps` once `napkin-core` passes its own lints.
3. Optional: match the settings window's native title bar to the theme appearance.

## Decisions

- **Built-in editor, no custom text element.** `EditorState` gives IME, selection, undo,
  clipboard and scrolling for free. Its hooks are enough:
  - Highlighting: `set_highlighter_factory` + `InputHighlighter::styles(range, resolver)`. The
    highlighter re-evaluates the whole document in `update` (it receives the full rope), so
    spans are always in step with the text. `Sheet::runs` clamps spans to the line and to char
    boundaries and covers the requested range without gaps.
  - Result column: `set_editor_paddings` (doc-hidden but public) gives the editor a right
    padding of `column + gap + inset`; the text element shrinks, so soft wrap stops before the
    column. The overlay is a sibling painted *after* the editor. The editor publishes its line
    layout (`last_layout`, `last_bounds`) at the end of its own paint, so in the overlay's paint
    `EditorState::range_to_bounds(line_start..line_start)` is the current frame's position of
    the line's first visual row, scroll included. No frame of lag.
  - Hit testing: one `Hitbox` over the overlay in prepaint; in paint, mouse listeners compare
    the event position with this frame's result bounds, copy on click and stop propagation so
    the editor does not move the caret. Hover changes call `window.refresh()`.
- **Column width = widest result in the whole document** (capped at 50 % of the editor width,
  longer results are cut with an ellipsis, the full value still copies). The spec says "widest
  visible result"; sizing by visible lines would re-wrap the text while scrolling, which looks
  jumpy. Documents are small, so the document-wide width is the calmer choice.
- The view renders the editor entity directly, not gpui-component's `Input` wrapper, because
  the wrapper re-projects paddings and editor style on every render.
- Font: the system font by default (Numi 3.22 "use system font"), Menlo as the monospace option.
- New/opened notes put the caret at the end (a scratchpad continues where it ends).
- **Notes:** timestamp file names (`note-YYYYMMDD-HHMMSS.numi`) title themselves from their
  first non-empty line; renaming renames the file and the stem becomes the title (an empty name
  restores the automatic title). Order = creation time, newest first, everywhere (list and
  `⌘⇧]` = next = down the list). Saving writes in place because an atomic rename would reset the
  creation time. Empty notes are removed when you move away from them. Delete goes to the Trash.
- **Closing the window hides Napkin** (it keeps running for the menu bar icon and hotkey, like
  Numi); `⌘Q` quits and saves.
- **Actions are global** (`cx.on_action`) and run on the main window through `cx.defer`: a
  global handler cannot update the window that is dispatching (it is taken out of the window
  map during dispatch), and this way menu items and keys work from the settings window too.
- **Menu bar icon: `tray-icon`, not `gpui-tray`.** `gpui-tray` 0.1.4 hands AppKit an image that
  is not a template and is sized in pixels; `tray-icon` scales to 18 pt and supports template
  images. Left click toggles the window, right click opens Show / New Note / Settings / Quit.
  Its event handlers are process-global and set once, so the icon is installed once and only
  shown/hidden afterwards.
- **Launch at login: a LaunchAgent plist** (`~/Library/LaunchAgents/com.neuhaus.napkin.plist`,
  `open -a <bundle>` inside a bundle, the binary otherwise, rewritten on each launch). Works
  unsigned and for dev binaries; `SMAppService` would need a signed bundle.
- **Print:** RTF (results at a right-aligned tab stop) in an off-screen `NSTextView`, printed
  with `NSPrintOperation::runOperationModalForWindow` (a sheet, so no nested modal run loop
  inside a GPUI callback).
- **Alfred endpoint:** answers `200 text/plain` with the result, an empty body when the
  expression computes nothing; binds only `127.0.0.1:15055`; its thread reads the shared
  `Arc<RwLock<Engine>>`, so settings, rates and extensions apply to it too.
- **Copy all** writes `input = result` (single spaces, like Numi's Alfred "workings").
- **Reload extensions** builds a fresh engine (units/functions cannot be unregistered) with the
  current settings and rates.
- Settings are flat JSON with nested `engine` (napkin-core `Settings`) and `desktop`; booleans
  in function signatures are `settings::Switch` (BeautifulRust: no bool parameters).
- Theme colours are CSS hex strings parsed by a tested helper (clippy's grouped hex literals
  are unreadable for colours).

## Gotchas (gpui-kit 0.7.3, this Mac)

- `cargo clippy -p napkin-app -- -D warnings` also lints workspace path dependencies; the stub
  `napkin-core` fails it (not ours). Use `--no-deps` for this crate's gate.
- `Timer` is not exported; use `cx.background_executor().timer(duration)`.
- `AsyncApp::update` returns the closure's value directly (no `Result`); `AsyncWindowContext`
  still returns a `Result`.
- `gpui_kit::open_window` returns the `Root` wrapper's window handle; keep the returned view
  entity next to it (`app::MainWindow`) instead of downcasting the handle.
- `Application::on_reopen` / `on_open_urls` are on the `Application` (before `run`), and
  `on_open_urls` gets no `App`: forward the URLs through a channel.
- In edition 2024 an `-> impl IntoElement` helper that takes `&Napkin` keeps `cx` borrowed;
  return a concrete `Div` when the caller needs `cx` mutably afterwards.
- The view's `render` runs outside the editor's text-style scope, so `window.text_style()` there
  is the root style. Pass the editor font explicitly to anything measuring text in `render`.
- `Window::window_handle()` (GPUI's own) shadows `HasWindowHandle::window_handle`; call the
  trait method explicitly for the NSView.
- Python is blocked by a local hook; use Edit/sed/perl.
- macOS 26 hosts status items out of process: a status item (even a plain AppKit one) has no
  window under the app's pid in `CGWindowList`, so the menu bar icon cannot be verified that way.
- Testing on Marc's Mac: launch with `NAPKIN_BACKGROUND=1` (no activation, unfocused windows)
  and `NAPKIN_HOME=tmp/...` with `hotkey: null`, `menu_bar_icon: false`, `alfred_http: false`,
  or keystrokes typed by Marc land in the test window and system-wide registrations collide.
  Synthetic events posted to the pid do not reach an inactive window; use
  `NAPKIN_TEST_ACTIONS=napkin::ToggleNotesList` (or any registered action) for UI states.
- GPUI stops a window's display link while AppKit reports it occluded. An unfocused test window
  behind Marc's windows therefore shows its first frame only; later state changes (test actions)
  are real but not painted. Verify state changes through files (`notes/`, `state.json`) then,
  and trust captures only when the window was drawn after the change.
- Test helpers live in `tmp/tools/` (not committed): `winid` (window id + lock state),
  `poke` (pid-targeted events), `shot.sh` (launch in background, capture, quit), `allwin`,
  `statusitems`. Richer highlighting screenshots used a throwaway span fake copied in from
  `tmp/fake/` and removed again (never committed).
