//! The menu bar icon: a click toggles the window, a right click opens a small menu.

use anyhow::Context as _;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Side of the generated template icon in pixels (drawn at 18 pt on a 2x display).
const ICON_SIZE: u32 = 36;

/// What the menu bar icon asks the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayCommand {
    ToggleWindow,
    NewNote,
    ShowSettings,
    Quit,
}

impl TrayCommand {
    const ALL: [Self; 4] = [Self::ToggleWindow, Self::NewNote, Self::ShowSettings, Self::Quit];

    fn id(self) -> &'static str {
        match self {
            Self::ToggleWindow => "napkin.toggle",
            Self::NewNote => "napkin.new-note",
            Self::ShowSettings => "napkin.settings",
            Self::Quit => "napkin.quit",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ToggleWindow => "Show Napkin",
            Self::NewNote => "New Note",
            Self::ShowSettings => "Settings…",
            Self::Quit => "Quit Napkin",
        }
    }
}

/// The installed menu bar icon. Dropping it removes the icon.
pub(crate) struct MenuBarIcon {
    tray: TrayIcon,
}

impl std::fmt::Debug for MenuBarIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuBarIcon").finish_non_exhaustive()
    }
}

impl MenuBarIcon {
    /// Installs the icon. Commands arrive on `commands`; tray-icon's event handlers are global
    /// and can be set only once per process, so call this once and use [`Self::set_visible`].
    pub(crate) fn install(commands: async_channel::Sender<TrayCommand>) -> anyhow::Result<Self> {
        let menu = Menu::new();
        for (index, command) in TrayCommand::ALL.into_iter().enumerate() {
            if index == 3 {
                menu.append(&PredefinedMenuItem::separator())
                    .context("building the tray menu")?;
            }
            menu.append(&MenuItem::with_id(command.id(), command.label(), true, None))
                .context("building the tray menu")?;
        }
        let icon = Icon::from_rgba(template_icon(), ICON_SIZE, ICON_SIZE).context("drawing the tray icon")?;
        let tray = TrayIconBuilder::new()
            .with_id("napkin")
            .with_icon_templated(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("Napkin")
            .build()
            .context("creating the menu bar icon")?;

        let clicks = commands.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = clicks.try_send(TrayCommand::ToggleWindow);
            }
        }));
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some(command) = TrayCommand::ALL
                .into_iter()
                .find(|command| event.id().0 == command.id())
            {
                let _ = commands.try_send(command);
            }
        }));
        Ok(Self { tray })
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        if let Err(error) = self.tray.set_visible(visible) {
            eprintln!("could not change the menu bar icon: {error}");
        }
    }
}

/// A sheet of paper with an equals sign, as a black-on-transparent template image (RGBA).
fn template_icon() -> Vec<u8> {
    let size = f64::from(ICON_SIZE);
    let mut rgba = Vec::with_capacity(usize::try_from(ICON_SIZE * ICON_SIZE * 4).unwrap_or_default());
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let point = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let sheet = rounded_rect_distance(point, (size / 2., size / 2.), (12.5, 15.5), 4.).abs() - 1.4;
            let bar = |center_y: f64| capsule_distance(point, (12.5, center_y), (23.5, center_y), 1.5);
            let ink = sheet.min(bar(15.)).min(bar(21.));
            let coverage = (0.5 - ink).clamp(0., 1.);
            rgba.extend_from_slice(&[0, 0, 0, unit_to_byte(coverage)]);
        }
    }
    rgba
}

/// Maps 0..=1 to 0..=255.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 before the cast"
)]
fn unit_to_byte(value: f64) -> u8 {
    (value.clamp(0., 1.) * 255.).round() as u8
}

/// Signed distance from `point` to a rounded rectangle.
fn rounded_rect_distance(point: (f64, f64), center: (f64, f64), half: (f64, f64), radius: f64) -> f64 {
    let qx = (point.0 - center.0).abs() - half.0 + radius;
    let qy = (point.1 - center.1).abs() - half.1 + radius;
    let outside = qx.max(0.).hypot(qy.max(0.));
    outside + qx.max(qy).min(0.) - radius
}

/// Signed distance from `point` to a capsule from `a` to `b` with the given radius.
fn capsule_distance(point: (f64, f64), a: (f64, f64), b: (f64, f64), radius: f64) -> f64 {
    let (pax, pay) = (point.0 - a.0, point.1 - a.1);
    let (bax, bay) = (b.0 - a.0, b.1 - a.1);
    let t = ((pax * bax + pay * bay) / (bax * bax + bay * bay)).clamp(0., 1.);
    (pax - bax * t).hypot(pay - bay * t) - radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_icon_has_ink_and_transparency() {
        let icon = template_icon();
        assert_eq!(icon.len(), 36 * 36 * 4);
        let alphas: Vec<u8> = icon.chunks(4).map(|pixel| pixel[3]).collect();
        assert!(alphas.contains(&255), "some fully inked pixels");
        assert!(alphas.contains(&0), "some transparent pixels");
        assert!(
            icon.chunks(4).all(|pixel| pixel[..3] == [0, 0, 0]),
            "template icons are black"
        );
    }

    #[test]
    fn bytes_cover_the_unit_range() {
        assert_eq!(unit_to_byte(0.), 0);
        assert_eq!(unit_to_byte(1.), 255);
        assert_eq!(unit_to_byte(0.5), 128);
    }
}
