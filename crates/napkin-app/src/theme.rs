//! Napkin's colour themes: one palette per theme with a colour for every [`SpanKind`], the
//! highlight resolver the editor reads them through, and the projection onto gpui-component.

use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::base::input::HighlightStyleResolver;
use gpui_kit::component::{Theme as KitTheme, ThemeMode};
use gpui_kit::{App, FontStyle, FontWeight, HighlightStyle, Hsla, WindowAppearance, rgb, rgba};
use napkin_core::SpanKind;
use serde::{Deserialize, Serialize};

use crate::sheet::kind_name;

/// The theme the user picked in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum ThemeChoice {
    /// Light or Dark, following the system appearance.
    #[default]
    System,
    Light,
    Dark,
    /// Dark grey with warm amber results. Settings written before 0.1.2 call it `NumiClassic`.
    #[serde(alias = "NumiClassic")]
    Amber,
    SolarizedLight,
    SolarizedDark,
}

impl ThemeChoice {
    pub(crate) const ALL: [Self; 6] = [
        Self::System,
        Self::Light,
        Self::Dark,
        Self::Amber,
        Self::SolarizedLight,
        Self::SolarizedDark,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Amber => "Amber",
            Self::SolarizedLight => "Solarized Light",
            Self::SolarizedDark => "Solarized Dark",
        }
    }

    /// The palette to paint with under the given system appearance.
    pub(crate) fn palette(self, appearance: WindowAppearance) -> Palette {
        match self {
            Self::System => match appearance {
                WindowAppearance::Dark | WindowAppearance::VibrantDark => dark(),
                WindowAppearance::Light | WindowAppearance::VibrantLight => light(),
            },
            Self::Light => light(),
            Self::Dark => dark(),
            Self::Amber => amber(),
            Self::SolarizedLight => solarized_light(),
            Self::SolarizedDark => solarized_dark(),
        }
    }
}

/// Colours for each syntax category.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SpanColors {
    pub(crate) number: Hsla,
    pub(crate) unit: Hsla,
    pub(crate) currency: Hsla,
    pub(crate) operator: Hsla,
    pub(crate) keyword: Hsla,
    pub(crate) variable: Hsla,
    pub(crate) function: Hsla,
    pub(crate) constant: Hsla,
    pub(crate) date_time: Hsla,
    pub(crate) comment: Hsla,
    pub(crate) header: Hsla,
    pub(crate) label: Hsla,
    pub(crate) text: Hsla,
}

impl SpanColors {
    pub(crate) fn of(&self, kind: SpanKind) -> Hsla {
        match kind {
            SpanKind::Number => self.number,
            SpanKind::Unit => self.unit,
            SpanKind::Currency => self.currency,
            SpanKind::Operator => self.operator,
            SpanKind::Keyword => self.keyword,
            SpanKind::Variable => self.variable,
            SpanKind::Function => self.function,
            SpanKind::Constant => self.constant,
            SpanKind::DateTime => self.date_time,
            SpanKind::Comment => self.comment,
            SpanKind::Header => self.header,
            SpanKind::Label => self.label,
            SpanKind::Text => self.text,
        }
    }
}

/// Every colour the app paints with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Palette {
    pub(crate) dark: bool,
    pub(crate) background: Hsla,
    pub(crate) text: Hsla,
    /// Secondary text: the title, hints, list details.
    pub(crate) muted: Hsla,
    pub(crate) result: Hsla,
    /// Background behind a hovered result.
    pub(crate) result_hover: Hsla,
    /// Colour of the "Copied" flash.
    pub(crate) accent: Hsla,
    pub(crate) selection: Hsla,
    pub(crate) caret: Hsla,
    pub(crate) border: Hsla,
    /// Background of panels laid over the editor (notes list).
    pub(crate) panel: Hsla,
    pub(crate) spans: SpanColors,
}

impl Palette {
    /// The resolver the editor maps highlight names through.
    pub(crate) fn highlight_styles(&self) -> Arc<dyn HighlightStyleResolver> {
        const KINDS: [SpanKind; 13] = [
            SpanKind::Number,
            SpanKind::Unit,
            SpanKind::Currency,
            SpanKind::Operator,
            SpanKind::Keyword,
            SpanKind::Variable,
            SpanKind::Function,
            SpanKind::Constant,
            SpanKind::DateTime,
            SpanKind::Comment,
            SpanKind::Header,
            SpanKind::Label,
            SpanKind::Text,
        ];
        let styles = KINDS
            .into_iter()
            .map(|kind| {
                let mut style = HighlightStyle {
                    color: Some(self.spans.of(kind)),
                    ..HighlightStyle::default()
                };
                match kind {
                    SpanKind::Header => style.font_weight = Some(FontWeight::BOLD),
                    SpanKind::Comment => style.font_style = Some(FontStyle::Italic),
                    _ => {}
                }
                (kind_name(kind), style)
            })
            .collect();
        Arc::new(SpanStyles { styles })
    }

    /// Projects the palette onto gpui-component, so inputs, lists and the settings window match.
    pub(crate) fn apply_to_components(&self, cx: &mut App) {
        let mode = if self.dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        if KitTheme::global(cx).mode != mode {
            KitTheme::change(mode, None, cx);
        }
        let palette = *self;
        KitTheme::update(cx, |theme| {
            theme.colors.background = palette.background;
            theme.colors.foreground = palette.text;
            theme.colors.muted_foreground = palette.muted;
            theme.colors.border = palette.border;
            theme.colors.input = palette.border;
            theme.colors.selection = palette.selection;
            theme.colors.caret = palette.caret;
            theme.colors.popover = palette.panel;
            theme.colors.popover_foreground = palette.text;
            theme.colors.list_active = palette.result_hover;
            theme.colors.list_hover = palette.result_hover.opacity(0.6);
            theme.colors.ring = palette.accent;
            theme.colors.primary = palette.accent;
            theme.colors.primary_hover = palette.accent.opacity(0.88);
            theme.colors.primary_active = palette.accent;
            theme.colors.primary_foreground = palette.background;
            theme.colors.switch = palette.border;
        });
    }
}

/// [`HighlightStyleResolver`] over a fixed table of highlight names.
struct SpanStyles {
    styles: HashMap<&'static str, HighlightStyle>,
}

impl HighlightStyleResolver for SpanStyles {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        self.styles.get(name).copied()
    }
}

/// An opaque colour written like CSS: `"#26282d"`.
fn color(hex: &str) -> Hsla {
    rgb(parse_hex(hex)).into()
}

/// A colour with alpha written like CSS: `"#2f7d681f"`.
fn translucent(hex: &str) -> Hsla {
    rgba(parse_hex(hex)).into()
}

fn parse_hex(hex: &str) -> u32 {
    u32::from_str_radix(hex.trim_start_matches('#'), 16)
        .expect("theme colours are valid hex (every theme is built in the tests)")
}

fn light() -> Palette {
    let text = color("#26282d");
    Palette {
        dark: false,
        background: color("#fcfcfb"),
        text,
        muted: color("#8c9098"),
        result: color("#2f7d68"),
        result_hover: translucent("#2f7d681f"),
        accent: color("#2f6feb"),
        selection: translucent("#8ab6ff59"),
        caret: color("#2f6feb"),
        border: color("#e6e6e3"),
        panel: color("#ffffff"),
        spans: SpanColors {
            number: text,
            unit: color("#2b74c0"),
            currency: color("#2b74c0"),
            operator: color("#848992"),
            keyword: color("#8a5cc2"),
            variable: color("#b44f8c"),
            function: color("#c06a1c"),
            constant: color("#c06a1c"),
            date_time: color("#238a8a"),
            comment: color("#a2a6ad"),
            header: text,
            label: color("#9a9ea6"),
            text: text.opacity(0.62),
        },
    }
}

fn dark() -> Palette {
    let text = color("#e3e4e6");
    Palette {
        dark: true,
        background: color("#1d1e21"),
        text,
        muted: color("#7d8189"),
        result: color("#7ccfae"),
        result_hover: translucent("#7ccfae24"),
        accent: color("#8ab4ff"),
        selection: translucent("#4d7cd666"),
        caret: color("#8ab4ff"),
        border: color("#303237"),
        panel: color("#25272b"),
        spans: SpanColors {
            number: text,
            unit: color("#7ab4f0"),
            currency: color("#7ab4f0"),
            operator: color("#8d929b"),
            keyword: color("#b996ec"),
            variable: color("#e48dc2"),
            function: color("#e6a76a"),
            constant: color("#e6a76a"),
            date_time: color("#6ccbcb"),
            comment: color("#6c7078"),
            header: color("#f4f5f6"),
            label: color("#80848c"),
            text: text.opacity(0.6),
        },
    }
}

fn amber() -> Palette {
    let text = color("#e8e6e1");
    Palette {
        dark: true,
        background: color("#2b2b2b"),
        text,
        muted: color("#868480"),
        result: color("#f5c451"),
        result_hover: translucent("#f5c45126"),
        accent: color("#f5c451"),
        selection: translucent("#6b655799"),
        caret: color("#f5c451"),
        border: color("#3a3a3a"),
        panel: color("#333333"),
        spans: SpanColors {
            number: text,
            unit: color("#8fc8e8"),
            currency: color("#8fc8e8"),
            operator: color("#9a9792"),
            keyword: color("#c9a8e8"),
            variable: color("#e8a080"),
            function: color("#a8d88c"),
            constant: color("#a8d88c"),
            date_time: color("#8cd8c8"),
            comment: color("#777570"),
            header: color("#f4f2ee"),
            label: color("#8a8883"),
            text: text.opacity(0.6),
        },
    }
}

fn solarized_light() -> Palette {
    let body = color("#657b83"); // base00
    let emphasis = color("#586e75"); // base01
    let subtle = color("#93a1a1"); // base1
    Palette {
        dark: false,
        background: color("#fdf6e3"),
        text: body,
        muted: subtle,
        result: color("#859900"),
        result_hover: translucent("#85990022"),
        accent: color("#268bd2"),
        selection: translucent("#d8cfb0cc"),
        caret: color("#268bd2"),
        border: color("#eee8d5"),
        panel: color("#eee8d5"),
        spans: SpanColors {
            number: emphasis,
            unit: color("#268bd2"),
            currency: color("#268bd2"),
            operator: subtle,
            keyword: color("#6c71c4"),
            variable: color("#d33682"),
            function: color("#cb4b16"),
            constant: color("#cb4b16"),
            date_time: color("#2aa198"),
            comment: subtle,
            header: color("#073642"),
            label: subtle,
            text: body.opacity(0.75),
        },
    }
}

fn solarized_dark() -> Palette {
    let body = color("#839496"); // base0
    let emphasis = color("#93a1a1"); // base1
    let subtle = color("#586e75"); // base01
    Palette {
        dark: true,
        background: color("#002b36"),
        text: body,
        muted: subtle,
        result: color("#b58900"),
        result_hover: translucent("#b5890026"),
        accent: color("#268bd2"),
        selection: translucent("#0a4d5eff"),
        caret: color("#268bd2"),
        border: color("#073642"),
        panel: color("#073642"),
        spans: SpanColors {
            number: emphasis,
            unit: color("#268bd2"),
            currency: color("#268bd2"),
            operator: subtle,
            keyword: color("#6c71c4"),
            variable: color("#d33682"),
            function: color("#cb4b16"),
            constant: color("#cb4b16"),
            date_time: color("#2aa198"),
            comment: subtle,
            header: color("#eee8d5"),
            label: subtle,
            text: body.opacity(0.75),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_theme_name_still_loads() {
        let theme: ThemeChoice = serde_json::from_str("\"NumiClassic\"").expect("old name parses");
        assert_eq!(theme, ThemeChoice::Amber);
        assert_eq!(serde_json::to_string(&theme).expect("serializes"), "\"Amber\"");
    }

    #[test]
    fn every_theme_resolves_every_span_kind() {
        for choice in ThemeChoice::ALL {
            let palette = choice.palette(WindowAppearance::Light);
            let styles = palette.highlight_styles();
            for kind in [
                SpanKind::Number,
                SpanKind::Header,
                SpanKind::Text,
                SpanKind::Comment,
            ] {
                let style = styles.style(kind_name(kind));
                assert_eq!(
                    style.and_then(|style| style.color),
                    Some(palette.spans.of(kind)),
                    "{choice:?}"
                );
            }
        }
    }

    #[test]
    fn system_follows_the_appearance() {
        assert!(ThemeChoice::System.palette(WindowAppearance::Dark).dark);
        assert!(!ThemeChoice::System.palette(WindowAppearance::Light).dark);
    }
}
