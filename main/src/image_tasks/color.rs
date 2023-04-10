use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Mul;

use tiny_skia::Color;
use tiny_skia::ColorU8;
use tiny_skia::PremultipliedColor;
use tiny_skia::PremultipliedColorU8;

/// Wrapper around [ColorU8] that implements important missing traits such as [Eq], [Hash], [Copy],
/// [Clone] and [Ord]. Represents a 24-bit sRGB color + 8-bit alpha value (not premultiplied).
#[derive(Eq, Debug, Copy, Clone, Ord, PartialOrd)]
pub struct ComparableColor {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl ComparableColor {
    pub fn red(&self) -> u8 { return self.red; }
    pub fn green(&self) -> u8 { return self.green; }
    pub fn blue(&self) -> u8 { return self.blue; }
    pub fn alpha(&self) -> u8 { return self.alpha; }

    pub const TRANSPARENT: ComparableColor = rgba(0,0,0,0);
    pub const BLACK: ComparableColor = gray(0);
    pub const RED: ComparableColor = rgb(u8::MAX,0,0);
    pub const GREEN: ComparableColor = rgb(0,u8::MAX,0);
    pub const BLUE: ComparableColor = rgb(0,0,u8::MAX);
    pub const YELLOW: ComparableColor = rgb(u8::MAX,u8::MAX,0);
    pub const MAGENTA: ComparableColor = rgb(u8::MAX,0,u8::MAX);
    pub const CYAN: ComparableColor = rgb(0,u8::MAX,u8::MAX);
    pub const WHITE: ComparableColor = gray(u8::MAX);

    pub const STONE_EXTREME_SHADOW: ComparableColor = gray(0x51);
    pub const STONE_SHADOW: ComparableColor = gray(0x74);
    pub const STONE: ComparableColor = gray(0x85);
    pub const STONE_HIGHLIGHT: ComparableColor = gray(0xaa);
    pub const STONE_EXTREME_HIGHLIGHT: ComparableColor = gray(0xba);

    pub const EXTRA_DARK_BIOME_COLORABLE: ComparableColor = ComparableColor::STONE_SHADOW;
    pub const DARK_BIOME_COLORABLE: ComparableColor = ComparableColor::STONE;
    pub const MEDIUM_BIOME_COLORABLE: ComparableColor = gray(0x9d);
    pub const LIGHT_BIOME_COLORABLE: ComparableColor = ComparableColor::STONE_EXTREME_HIGHLIGHT;
    pub const EXTRA_LIGHT_BIOME_COLORABLE: ComparableColor = gray(0xc3);
}

impl Mul<f32> for ComparableColor {
    type Output = ComparableColor;

    fn mul(self, rhs: f32) -> Self::Output {
        let out_alpha = f32::from(self.alpha) * rhs;
        return ComparableColor {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha: out_alpha as u8
        }
    }
}

impl Display for ComparableColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.alpha == 0 {
            write!(f, "transparent")
        } else {
            write!(f, "#{:02x}{:02x}{:02x}{:02x}", self.red, self.green, self.blue, self.alpha)
        }
    }
}

const CHANNEL_MAX_F32: f32 = u8::MAX as f32;

impl From<Color> for ComparableColor {
    fn from(value: Color) -> Self {
        return ComparableColor {
            red: (value.red() * CHANNEL_MAX_F32) as u8,
            green: (value.green() * CHANNEL_MAX_F32) as u8,
            blue: (value.blue() * CHANNEL_MAX_F32) as u8,
            alpha: (value.alpha() * CHANNEL_MAX_F32) as u8,
        };
    }
}

impl From<PremultipliedColor> for ComparableColor {
    fn from(value: PremultipliedColor) -> Self {
        return ComparableColor::from(value.demultiply());
    }
}

impl From<ColorU8> for ComparableColor {
    fn from(value: ColorU8) -> Self {
        return ComparableColor {
            red: value.red(),
            green: value.green(),
            blue: value.blue(),
            alpha: value.alpha(),
        };
    }
}

impl From<PremultipliedColorU8> for ComparableColor {
    fn from(value: PremultipliedColorU8) -> Self {
        return ComparableColor::from(value.demultiply());
    }
}

impl Into<Color> for ComparableColor {
    fn into(self) -> Color {
        return Color::from_rgba8(self.red, self.green, self.blue, self.alpha);
    }
}

impl Into<PremultipliedColor> for ComparableColor {
    fn into(self) -> PremultipliedColor {
        let color: Color = self.into();
        return color.premultiply();
    }
}

impl Into<ColorU8> for ComparableColor {
    fn into(self) -> ColorU8 {
        return ColorU8::from_rgba(self.red, self.green, self.blue, self.alpha);
    }
}

impl Into<PremultipliedColorU8> for ComparableColor {
    fn into(self) -> PremultipliedColorU8 {
        let color: ColorU8 = self.into();
        return color.premultiply();
    }
}

impl PartialEq<Self> for ComparableColor {
    fn eq(&self, other: &Self) -> bool {
        return (self.alpha == 0 && other.alpha == 0) ||
            (self.red == other.red
                && self.green == other.green
                && self.blue == other.blue
                && self.alpha == other.alpha);
    }
}

#[test]
fn test_eq() {
    assert_eq!(ComparableColor::BLACK, ComparableColor::BLACK);
    assert_eq!(ComparableColor::RED, ComparableColor::RED);
    assert_eq!(ComparableColor::GREEN, ComparableColor::GREEN);
    assert_eq!(ComparableColor::BLUE, ComparableColor::BLUE);
    assert_eq!(ComparableColor::WHITE, ComparableColor::WHITE);
    assert_eq!(ComparableColor::TRANSPARENT, ComparableColor::TRANSPARENT);

    assert_ne!(ComparableColor::BLACK, ComparableColor::RED);
    assert_ne!(ComparableColor::BLACK, ComparableColor::GREEN);
    assert_ne!(ComparableColor::BLACK, ComparableColor::BLUE);
    assert_ne!(ComparableColor::BLACK, ComparableColor::WHITE);
    assert_ne!(ComparableColor::BLACK, ComparableColor::TRANSPARENT);

    // When alpha is zero (totally transparent), the color values don't matter
    assert_eq!(rgba(0,0,0,0),rgba(u8::MAX, u8::MAX, u8::MAX, 0));
}

impl Hash for ComparableColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.alpha.hash(state);
        if self.alpha != 0 {
            self.red.hash(state);
            self.green.hash(state);
            self.blue.hash(state);
        }
    }
}

#[test]
fn test_hash() {
    fn hash(color: ComparableColor) -> u64 {
        let mut hasher = DefaultHasher::new();
        color.hash(&mut hasher);
        hasher.finish()
    }
    let black_hash = hash(ComparableColor::BLACK);
    let red_hash = hash(ComparableColor::RED);
    let green_hash = hash(ComparableColor::GREEN);
    let blue_hash = hash(ComparableColor::BLUE);
    let transparent_hash_1 = hash(rgba(0,0,0,0));
    let transparent_hash_2 = hash(rgba(u8::MAX, u8::MAX, u8::MAX, 0));

    assert_ne!(black_hash, red_hash);
    assert_ne!(black_hash, green_hash);
    assert_ne!(black_hash, blue_hash);
    assert_ne!(black_hash, transparent_hash_1);
    assert_ne!(black_hash, transparent_hash_2);
    assert_ne!(red_hash, green_hash);
    assert_ne!(red_hash, blue_hash);
    assert_ne!(green_hash, blue_hash);

    // When alpha is zero (totally transparent), the color values don't matter
    assert_eq!(transparent_hash_1,transparent_hash_2);
}

pub const fn rgb(r: u8, g: u8, b: u8) -> ComparableColor {
    ComparableColor {
        red: r, green: g, blue: b, alpha: u8::MAX
    }
}

pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> ComparableColor {
    ComparableColor {
        red: r, green: g, blue: b, alpha: a
    }
}

pub const fn gray(lightness: u8) -> ComparableColor {
    rgb(lightness, lightness, lightness)
}

#[test]
fn test_gray() {
    let gray = gray(0x7f);
    assert_eq!(gray.red, 0x7f);
    assert_eq!(gray.green, 0x7f);
    assert_eq!(gray.blue, 0x7f);
    assert_eq!(gray.alpha, u8::MAX);
}

pub const fn c(rgb: u32) -> ComparableColor {
    let bytes = rgb.to_be_bytes();
    ComparableColor {
        red: bytes[1],
        green: bytes[2],
        blue: bytes[3],
        alpha: u8::MAX
    }
}

#[test]
fn test_c() {
    assert_eq!(c(0xc0ffee),
               ComparableColor { red: 0xc0, green: 0xff, blue: 0xee, alpha: u8::MAX }
    )
}

impl ComparableColor {
    /** True if this color is black, transparent, or semitransparent black. */
    pub fn is_black_or_transparent(&self) -> bool {
        self.alpha == 0 || (self.red == 0 && self.green == 0 && self.blue == 0)
    }
}

#[test]
fn test_is_black_or_transparent() {
    assert!(ComparableColor::BLACK.is_black_or_transparent());
    assert!(ComparableColor::TRANSPARENT.is_black_or_transparent());
    assert!(rgba(0,0,0,0xcc).is_black_or_transparent()); // semitransparent black
    assert!(rgba(0xff,0xff,0xff,0).is_black_or_transparent()); // transparent but with r, g and b > 0

    assert!(!ComparableColor::RED.is_black_or_transparent());
    assert!(!ComparableColor::GREEN.is_black_or_transparent());
    assert!(!ComparableColor::BLUE.is_black_or_transparent());
    assert!(!(rgba(0xff,0x00,0x00,0xcc).is_black_or_transparent())); // semitransparent red
}

pub const fn ca(rgb: u32) -> ComparableColor {
    let bytes = rgb.to_be_bytes();
    ComparableColor {
        red: bytes[0],
        green: bytes[1],
        blue: bytes[2],
        alpha: bytes[3]
    }
}

#[test]
fn test_ca() {
    assert_eq!(ca(0x1337c0de),
               ComparableColor { red: 0x13, green: 0x37, blue: 0xc0, alpha: 0xde }
    )
}
