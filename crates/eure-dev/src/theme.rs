//! Catppuccin theme support for the Eure editor.

use catppuccin::{FlavorColors, Hex, PALETTE};
use eure_editor_support::semantic_token::SemanticTokenType;

/// Theme variants: Dark (Mocha) and Light (Latte).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    /// Toggle between Dark and Light themes.
    pub fn toggle(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    fn palette(self) -> FlavorColors {
        match self {
            Theme::Dark => PALETTE.mocha.colors,
            Theme::Light => PALETTE.latte.colors,
        }
    }

    /// Get the color for a semantic token type.
    pub fn token_color(self, token_type: SemanticTokenType) -> Hex {
        let p = self.palette();
        match token_type {
            SemanticTokenType::Keyword => p.mauve.hex,
            SemanticTokenType::Number => p.peach.hex,
            SemanticTokenType::String => p.green.hex,
            SemanticTokenType::Comment => p.overlay0.hex,
            SemanticTokenType::Operator => p.sky.hex,
            SemanticTokenType::Property => p.blue.hex,
            SemanticTokenType::Punctuation => p.text.hex,
            SemanticTokenType::Macro => p.teal.hex,
            SemanticTokenType::Decorator => p.yellow.hex,
            SemanticTokenType::SectionMarker => p.pink.hex,
            SemanticTokenType::ExtensionMarker => p.lavender.hex,
            SemanticTokenType::ExtensionIdent => p.flamingo.hex,
        }
    }

    /// Get the error underline color.
    pub fn error_color(self) -> Hex {
        self.palette().red.hex
    }

    /// Get the background color (for editors).
    pub fn bg_color(self) -> Hex {
        self.palette().base.hex
    }

    /// Get the page background color (slightly darker than editor bg).
    pub fn page_bg_color(self) -> Hex {
        self.palette().mantle.hex
    }

    /// Get the caret (cursor) color.
    pub fn caret_color(self) -> Hex {
        self.palette().text.hex
    }

    /// Get the text color (for non-highlighted text).
    pub fn text_color(self) -> Hex {
        self.palette().text.hex
    }

    /// Get the border color.
    pub fn border_color(self) -> Hex {
        self.palette().surface0.hex
    }

    /// Get the surface color (for tooltips, etc.).
    pub fn surface_color(self) -> Hex {
        self.palette().surface0.hex
    }

    /// Get the surface1 color (for buttons, toggles).
    pub fn surface1_color(self) -> Hex {
        self.palette().surface1.hex
    }

    /// Get the accent color (pink for emphasis).
    pub fn accent_color(self) -> Hex {
        self.palette().pink.hex
    }
}
