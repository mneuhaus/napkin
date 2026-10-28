//! Launch at login through a per-user launch agent (`~/Library/LaunchAgents/com.neuhaus.napkin.plist`).
//!
//! A launch agent works for the bundled app and for a bare development binary alike and needs no
//! signing or entitlements. Inside a bundle the agent starts the app with `open`, so Launch
//! Services launches it like a double click; the path is rewritten on every launch, so a moved
//! app keeps starting.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;

use crate::settings::{Switch, write_atomically};

/// The agent's label and file name.
pub(crate) const LABEL: &str = "com.neuhaus.napkin";

/// The agent plist in `launch_agents`.
pub(crate) fn agent_path(launch_agents: &Path) -> PathBuf {
    launch_agents.join(format!("{LABEL}.plist"))
}

/// Installs or removes the agent so it matches `enabled`.
pub(crate) fn apply(launch_agents: &Path, enabled: Switch) -> anyhow::Result<()> {
    let path = agent_path(launch_agents);
    if !enabled.is_on() {
        return match fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(error).with_context(|| format!("removing {}", path.display()))
            }
            _ => Ok(()),
        };
    }
    let executable = std::env::current_exe().context("locating the app")?;
    write_atomically(&path, agent_plist(&program_arguments(&executable)).as_bytes())
}

/// `open -a Napkin.app` inside a bundle, the executable itself otherwise.
fn program_arguments(executable: &Path) -> Vec<String> {
    let bundle = executable
        .ancestors()
        .find(|ancestor| ancestor.extension().is_some_and(|extension| extension == "app"));
    match bundle {
        Some(bundle) => vec!["/usr/bin/open".into(), "-a".into(), bundle.display().to_string()],
        None => vec![executable.display().to_string()],
    }
}

fn agent_plist(arguments: &[String]) -> String {
    let arguments = arguments.iter().fold(String::new(), |mut list, argument| {
        let _ = writeln!(list, "        <string>{}</string>", xml_escape(argument));
        list
    });
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
{arguments}    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundles_launch_through_open() {
        let arguments = program_arguments(Path::new("/Apps/Napkin.app/Contents/MacOS/napkin-app"));
        assert_eq!(arguments, ["/usr/bin/open", "-a", "/Apps/Napkin.app"]);
        let arguments = program_arguments(Path::new("/src/target/debug/napkin-app"));
        assert_eq!(arguments, ["/src/target/debug/napkin-app"]);
    }

    #[test]
    fn enabling_writes_and_disabling_removes_the_agent() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp/test-login-item");
        let _ = fs::remove_dir_all(&dir);
        apply(&dir, Switch::On).expect("installed");
        let plist = fs::read_to_string(agent_path(&dir)).expect("written");
        assert!(plist.contains("<string>com.neuhaus.napkin</string>"), "{plist}");
        assert!(plist.contains("<key>RunAtLoad</key>\n    <true/>"), "{plist}");
        apply(&dir, Switch::Off).expect("removed");
        assert!(!agent_path(&dir).exists());
        apply(&dir, Switch::Off).expect("removing twice is fine");
    }

    #[test]
    fn paths_are_escaped() {
        assert!(agent_plist(&["/a&b/<c>".into()]).contains("<string>/a&amp;b/&lt;c&gt;</string>"));
    }
}
