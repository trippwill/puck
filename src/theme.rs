// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! Puck's visual theme and shared widget styles.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};

/// Warm paper used for the application background.
pub const PAPER: Color = Color::from_rgb8(0xF5, 0xF2, 0xE9);
/// Near-black application text.
pub const INK: Color = Color::from_rgb8(0x19, 0x1A, 0x16);
/// Lime used for primary actions.
pub const LIME: Color = Color::from_rgb8(0xC7, 0xFF, 0x5B);
/// Mineral blue used for selections and source links.
pub const MINERAL_BLUE: Color = Color::from_rgb8(0x48, 0x67, 0xFF);
/// Pale coral used for warnings.
pub const PALE_CORAL: Color = Color::from_rgb8(0xFF, 0xB3, 0xA8);
/// Coral used for destructive actions and errors.
pub const CORAL: Color = Color::from_rgb8(0xFF, 0x72, 0x5E);

/// Returns the Puck Light theme.
#[must_use]
pub fn puck() -> Theme {
    Theme::custom(
        "Puck Light",
        iced::theme::Palette {
            background: PAPER,
            text: INK,
            primary: LIME,
            success: LIME,
            warning: PALE_CORAL,
            danger: CORAL,
        },
    )
}

/// Styles a primary action as a pill.
#[must_use]
pub fn primary_pill(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border.radius = 999.0.into();
    style
}

/// Styles a selected navigation row.
#[must_use]
pub fn selected(_theme: &Theme, status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Hovered => MINERAL_BLUE.scale_alpha(0.85),
        button::Status::Disabled => MINERAL_BLUE.scale_alpha(0.4),
        button::Status::Active | button::Status::Pressed => MINERAL_BLUE,
    };
    button::Style {
        background: Some(Background::Color(color)),
        text_color: Color::WHITE,
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Styles a quiet bordered panel.
#[must_use]
pub fn panel(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        text_color: Some(palette.background.base.text),
        border: Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puck_theme_uses_designed_palette() {
        let palette = puck().palette();

        assert_eq!(palette.background, PAPER);
        assert_eq!(palette.text, INK);
        assert_eq!(palette.primary, LIME);
        assert_eq!(palette.warning, PALE_CORAL);
        assert_eq!(palette.danger, CORAL);
    }
}
