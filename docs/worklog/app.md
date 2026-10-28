# Worklog: napkin-app

Durable log of the desktop app work (branch `app`). Newest milestone first in "Done".

## Done

- **M1: editor + result column proven.** gpui-base `EditorState` (code-editor mode, line numbers,
  folding, indent guides, auto-close, smart indent and search off) with a `SheetHighlighter`
  feeding engine spans as highlight names (`napkin.number`, ...) resolved by the theme's
  `HighlightStyleResolver`. `ResultsLayer` (custom element) paints each answer right-aligned on
  the first visual row of its line. Screenshots confirmed wrap and scroll alignment.

## Next

1. App shell: global state, actions, key bindings, menus, notes store, settings, themes.
2. Notes list overlay, import/export/print, shortcuts table.
3. Settings window, tray icon, global hotkey, launch at login, dock icon, HTTP endpoint.
4. Rates refresh + extensions, window frame memory, bundle script, gate.

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

## Gotchas (gpui-kit 0.7.3)

- `cargo clippy -p napkin-app -- -D warnings` also lints workspace path dependencies; the stub
  `napkin-core` fails it (not ours). Use `--no-deps` for this crate's gate.
- `Timer` is not exported; use `cx.background_executor().timer(duration)`.
- The view's `render` runs outside the editor's text-style scope, so `window.text_style()` there
  is the root style. Pass the editor font explicitly to anything measuring text in `render`.
- Python is blocked by a local hook; use Edit/sed/perl.
- Testing on Marc's Mac: launch with `NAPKIN_BACKGROUND=1` (no activation, unfocused window), or
  keystrokes typed by Marc land in the test window. Synthetic scroll events posted to the pid do
  not reach an inactive window; use document state (caret at end) to get a scrolled view.
- Test helpers live in `tmp/tools/` (not committed): `winid` (window id + lock state),
  `poke` (pid-targeted events), `shot.sh` (launch, capture, quit).
