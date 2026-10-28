//! The few macOS calls GPUI does not wrap: the Trash, the Dock icon, and the print panel.

use std::path::Path;

use gpui_kit::Window;

/// One line of a printout: what was typed and what it computes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrintLine {
    pub(crate) input: String,
    pub(crate) result: Option<String>,
}

/// Moves `path` to the Trash; `false` when that is not possible here.
pub(crate) fn move_to_trash(path: &Path) -> bool {
    imp::move_to_trash(path)
}

/// Shows or hides the Dock icon (and the app menu bar with it).
pub(crate) fn set_dock_icon_visible(visible: bool) {
    imp::set_dock_icon_visible(visible);
}

/// Opens the native print panel as a sheet on `window`, printing `lines` with results on the right.
pub(crate) fn print(title: &str, lines: &[PrintLine], window: &Window) -> anyhow::Result<()> {
    imp::print(title, &print_rtf(lines, imp::printable_width_points()), window)
}

/// An RTF document: one paragraph per line, the result at a right-aligned tab stop, in grey.
fn print_rtf(lines: &[PrintLine], width_points: f64) -> String {
    let tab_twips = format_twips(width_points);
    let mut rtf = String::from(
        "{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\fswiss Helvetica;}}\
         {\\colortbl;\\red40\\green40\\blue40;\\red96\\green96\\blue96;}\\f0\\fs22\n",
    );
    for line in lines {
        rtf.push_str(&format!("\\pard\\sl320\\tqr\\tx{tab_twips}\\cf1 "));
        rtf.push_str(&rtf_escape(&line.input));
        if let Some(result) = &line.result {
            rtf.push_str("\\tab\\cf2 ");
            rtf.push_str(&rtf_escape(result));
        }
        rtf.push_str("\\par\n");
    }
    rtf.push('}');
    rtf
}

/// Points as whole twips (1/20 point) for RTF.
fn format_twips(points: f64) -> String {
    format!("{:.0}", (points * 20.).max(0.))
}

/// Escapes RTF control characters and writes non-ASCII characters as `\uN?` (UTF-16 units).
fn rtf_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' | '{' | '}' => {
                escaped.push('\\');
                escaped.push(character);
            }
            '\t' => escaped.push_str("\\tab "),
            ' '..='~' => escaped.push(character),
            _ => {
                let mut units = [0_u16; 2];
                for unit in character.encode_utf16(&mut units) {
                    // RTF wants the UTF-16 unit as a signed 16-bit number.
                    escaped.push_str(&format!("\\u{}?", i16::from_ne_bytes(unit.to_ne_bytes())));
                }
            }
        }
    }
    escaped
}

#[cfg(target_os = "macos")]
mod imp {
    use std::path::Path;
    use std::ptr;

    use anyhow::Context as _;
    use gpui_kit::Window;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSPrintInfo, NSPrintOperation, NSTextView, NSView,
    };
    use objc2_foundation::{NSData, NSFileManager, NSPoint, NSRange, NSRect, NSSize, NSString, NSURL};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub(super) fn move_to_trash(path: &Path) -> bool {
        let Some(path) = path.to_str() else {
            return false;
        };
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        NSFileManager::defaultManager()
            .trashItemAtURL_resultingItemURL_error(&url, None)
            .is_ok()
    }

    pub(super) fn set_dock_icon_visible(visible: bool) {
        let Some(main_thread) = MainThreadMarker::new() else {
            return;
        };
        let policy = if visible {
            NSApplicationActivationPolicy::Regular
        } else {
            NSApplicationActivationPolicy::Accessory
        };
        NSApplication::sharedApplication(main_thread).setActivationPolicy(policy);
    }

    pub(super) fn printable_width_points() -> f64 {
        let info = NSPrintInfo::sharedPrintInfo();
        info.paperSize().width - info.leftMargin() - info.rightMargin()
    }

    pub(super) fn print(title: &str, rtf: &str, window: &Window) -> anyhow::Result<()> {
        let main_thread = MainThreadMarker::new().context("printing needs the main thread")?;
        let handle = HasWindowHandle::window_handle(window)
            .map_err(|error| anyhow::anyhow!("no native window: {error:?}"))?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            anyhow::bail!("not an AppKit window");
        };
        // SAFETY: GPUI hands out the content view of a live NSWindow; it stays valid for this
        // call because `window` is borrowed for its duration.
        let content_view: &NSView = unsafe { appkit.ns_view.cast::<NSView>().as_ref() };
        let ns_window = content_view.window().context("the view has no window")?;

        let width = printable_width_points();
        let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(width, 100.));
        let text_view = NSTextView::initWithFrame(main_thread.alloc(), frame);
        text_view.setEditable(false);
        text_view.setDrawsBackground(false);
        text_view.setHorizontallyResizable(false);
        text_view.setVerticallyResizable(true);
        text_view.replaceCharactersInRange_withRTF(NSRange::new(0, 0), &NSData::with_bytes(rtf.as_bytes()));
        text_view.sizeToFit();

        let operation = NSPrintOperation::printOperationWithView(&text_view);
        operation.setJobTitle(Some(&NSString::from_str(title)));
        operation.setShowsPrintPanel(true);
        // SAFETY: no delegate and no selector, so AppKit never calls back into us; the operation
        // retains the text view until the sheet closes.
        unsafe {
            operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                &ns_window,
                None,
                None,
                ptr::null_mut(),
            );
        }
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::path::Path;

    use gpui_kit::Window;

    pub(super) fn move_to_trash(_path: &Path) -> bool {
        false
    }

    pub(super) fn set_dock_icon_visible(_visible: bool) {}

    pub(super) fn printable_width_points() -> f64 {
        468.
    }

    pub(super) fn print(_title: &str, _rtf: &str, _window: &Window) -> anyhow::Result<()> {
        anyhow::bail!("printing is only available on macOS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtf_escapes_control_and_non_ascii_characters() {
        assert_eq!(rtf_escape(r"a\b{c}"), r"a\\b\{c\}");
        assert_eq!(rtf_escape("€5"), "\\u8364?5");
        assert_eq!(rtf_escape("😀"), "\\u-10179?\\u-8704?");
    }

    #[test]
    fn rtf_puts_results_at_a_right_tab() {
        let rtf = print_rtf(
            &[
                PrintLine {
                    input: "20 inches in cm".into(),
                    result: Some("50.8 cm".into()),
                },
                PrintLine {
                    input: "# Header".into(),
                    result: None,
                },
            ],
            468.,
        );
        assert!(
            rtf.contains("\\tqr\\tx9360\\cf1 20 inches in cm\\tab\\cf2 50.8 cm\\par"),
            "{rtf}"
        );
        assert!(rtf.contains("\\cf1 # Header\\par"), "{rtf}");
        assert!(rtf.starts_with("{\\rtf1") && rtf.ends_with('}'));
    }
}
