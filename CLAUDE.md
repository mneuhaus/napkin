# Napkin

Open-source calculator notepad in Rust with GPUI, compatible with Numi's syntax. Read
`docs/spec.md` first: it is the behaviour contract and lists which crate owns what.

## Rules

- Rust code follows BeautifulRust: read `.claude/skills/rust-best-practices/SKILL.md` before
  writing Rust here. Lints live in `Cargo.toml` (`[workspace.lints]`, library crates carry the
  full set in their own `[lints]`); `clippy.toml` is BeautifulRust's.
- Gate before handing back: `cargo fmt`, `cargo clippy -p <crate> --all-targets -- -D warnings`,
  `cargo test -p <crate>`. Fix lints instead of silencing them; a necessary suppression is
  `#[expect(clippy::name, reason = "...")]` on the narrowest item.
- Everything in the repo is English: code, comments, docs, commit messages, UI base language.
- Commits: plain English messages, no `Co-Authored-By` or other attribution lines.
- No secrets, personal data, screenshots or local paths in commits. Scratch files go in `tmp/`
  (gitignored), never in `/tmp`.
- The public APIs in `crates/napkin-core/src/lib.rs`, `crates/napkin-rates/src/lib.rs` and
  `crates/napkin-extensions/src/lib.rs` are cross-crate contracts: add, don't break.
- GUI testing on macOS: check the screen is unlocked before sending any synthetic input
  (`CGSessionCopyCurrentDictionary` → `CGSSessionScreenIsLocked`); locked means do not type.
