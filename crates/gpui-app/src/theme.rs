//! Palette, fonts and category colours.
//!
//! Same identity as the egui app: slate surfaces, sulfur yellow for what is
//! about to be deleted, and one mineral hue per category, shown as layers.

use cleaner_core::{Category, Risk};
use gpui::{rgb, Hsla, Rgba, SharedString};

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Hsla,
    pub surface: Hsla,
    pub raised: Hsla,
    pub line: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub accent: Hsla,
    pub accent_ink: Hsla,
    pub accent_text: Hsla,
    pub safe: Hsla,
    pub review: Hsla,
    pub danger: Hsla,
}

pub fn hex(v: u32) -> Hsla {
    rgb(v).into()
}

/// Opaque blend from `a` towards `b` (t = 0 gives `a`, 1 gives `b`).
pub fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let a: Rgba = a.into();
    let b: Rgba = b.into();
    let l = |x: f32, y: f32| x + (y - x) * t;
    Rgba { r: l(a.r, b.r), g: l(a.g, b.g), b: l(a.b, b.b), a: 1.0 }.into()
}

pub fn alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                dark,
                bg: hex(0x141922),
                surface: hex(0x1C232E),
                raised: hex(0x252E3B),
                line: hex(0x2F3947),
                text: hex(0xE8EBF1),
                muted: hex(0x8F99A9),
                accent: hex(0xF2C14E),
                accent_ink: hex(0xF2C14E),
                accent_text: hex(0x231A02),
                safe: hex(0x5CC8A8),
                review: hex(0xF0A04B),
                danger: hex(0xE8636C),
            }
        } else {
            Self {
                dark,
                bg: hex(0xECEEF2),
                surface: hex(0xFFFFFF),
                raised: hex(0xF3F5F8),
                line: hex(0xD7DCE3),
                text: hex(0x1A212B),
                muted: hex(0x5E6979),
                accent: hex(0xF2C14E),
                accent_ink: hex(0x9A6A00),
                accent_text: hex(0x231A02),
                safe: hex(0x1B9474),
                review: hex(0xB86512),
                danger: hex(0xC63F4A),
            }
        }
    }

    pub fn risk(&self, r: Risk) -> Hsla {
        match r {
            Risk::Safe => self.safe,
            Risk::Review => self.review,
            Risk::Blocked => self.danger,
        }
    }

    /// Soft tint of `c` on the current surface (pills, selected chips).
    pub fn tint(&self, c: Hsla) -> Hsla {
        mix(c, self.surface, if self.dark { 0.8 } else { 0.86 })
    }
}

pub fn category_color(c: Category) -> Hsla {
    hex(match c {
        Category::Npm => 0xE0564E,
        Category::Yarn => 0x4FA3D9,
        Category::Pnpm => 0xF09A3A,
        Category::Bun => 0xE8C99B,
        Category::Pip => 0x4C7FC0,
        Category::Uv => 0xB08CF0,
        Category::Cargo => 0xD07A4C,
        Category::Gradle => 0x3FB39B,
        Category::Maven => 0xC94C82,
        Category::Go => 0x4CC6DB,
        Category::NuGet => 0x7383DE,
        Category::Composer => 0xA88563,
        Category::Docker => 0x2F8FE6,
        Category::Wsl => 0x9BC53D,
        Category::Ide => 0xE07BF0,
        Category::Temp => 0x8E98A8,
    })
}

/// Text font: Segoe UI on Windows, the system UI font elsewhere.
pub fn body_font() -> SharedString {
    if cfg!(windows) { "Segoe UI".into() } else { ".SystemUIFont".into() }
}

/// Font for numbers and titles: Bahnschrift (a DIN-style face that ships
/// with Windows) or the body font elsewhere.
pub fn display_font() -> SharedString {
    if cfg!(windows) { "Bahnschrift".into() } else { body_font() }
}
