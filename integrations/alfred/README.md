# Napkin for Alfred

An [Alfred](https://www.alfredapp.com) workflow (Powerpack) that calculates with the `napkin`
command line tool: type `=20 inches in cm` or `napkin 20 inches in cm`.

- **Enter** copies the result, **⌘ Enter** copies `expression = result`, **⌥ Enter** copies the
  expression. **⌘ L** shows it in large type.
- Uses your extensions and the cached exchange rates, like the app.

## Install

1. Install the CLI: `cargo install --path crates/napkin-cli` (puts `napkin` in `~/.cargo/bin`).
2. Package and open the workflow from the repository root:

   ```sh
   mkdir -p tmp && (cd integrations/alfred && zip -q ../../tmp/Napkin.alfredworkflow info.plist script-filter.js)
   open tmp/Napkin.alfredworkflow
   ```

3. If `napkin` is not in `~/.cargo/bin`, `/opt/homebrew/bin` or `/usr/local/bin`, set its path
   in the workflow's configuration (**Configure Workflow…** → *napkin path*).

## Numi's workflow

Numi's own Alfred workflow talks to `http://localhost:15055/?q=<expression>`. It works unchanged
against Napkin: enable the Alfred integration in the app's settings, or run `napkin serve`.
