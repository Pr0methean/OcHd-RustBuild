use core::mem::transmute;
use oxipng::{BitDepth, RGBA8};
use palette::blend::Compose;
use palette::Srgba;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Mul;

use bytemuck::{cast, Pod, Zeroable};
use resvg::tiny_skia::Color;
use resvg::tiny_skia::ColorU8;
use resvg::tiny_skia::PremultipliedColor;
use resvg::tiny_skia::PremultipliedColorU8;

/// Wrapper around [ColorU8] that implements important missing traits such as [Eq], [Hash], [Copy],
/// [Clone] and [Ord]. Represents a 24-bit sRGB color + 8-bit alpha value (not premultiplied).
#[derive(Eq, Copy, Clone, Pod)]
#[repr(C)]
pub struct ComparableColor {
    pub(crate) alpha: u8,
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
}

unsafe impl Zeroable for ComparableColor {}

impl PartialOrd for ComparableColor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableColor {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.alpha == 0 && other.alpha == 0 {
            return Ordering::Equal;
        }
        let self_bytes: [u8; 4] = cast(*self);
        let other_bytes: [u8; 4] = cast(*other);
        self_bytes.cmp(&other_bytes)
    }
}

const fn bit_depth_for_channel_value() -> [BitDepth; u8::MAX as usize + 1] {
    let mut depth = [BitDepth::Eight; u8::MAX as usize + 1];
    let mut x = 1;
    while x <= 0xE {
        depth[x * 0x11] = BitDepth::Four;
        x += 1;
    }
    depth[0x55] = BitDepth::Two;
    depth[0xAA] = BitDepth::Two;
    depth[0] = BitDepth::One;
    depth[u8::MAX as usize] = BitDepth::One;
    depth
}

pub const BIT_DEPTH_FOR_CHANNEL: [BitDepth; u8::MAX as usize + 1] = bit_depth_for_channel_value();

impl ComparableColor {
    pub const fn red(&self) -> u8 {
        self.red
    }
    pub const fn green(&self) -> u8 {
        self.green
    }
    pub const fn blue(&self) -> u8 {
        self.blue
    }
    pub const fn alpha(&self) -> u8 {
        self.alpha
    }

    pub fn under<T>(self, foregrounds: T) -> Vec<ComparableColor>
    where
        T: Iterator<Item = ComparableColor>,
    {
        if self.alpha == 0 {
            foregrounds.collect()
        } else {
            let self_as_f32 = self.as_f32_srgba().premultiply();
            foregrounds
                .map(|fg_color| match fg_color.alpha() {
                    0 => self,
                    u8::MAX => fg_color,
                    _ => {
                        let foreground_as_f32 = fg_color.as_f32_srgba().premultiply();
                        let blended_as_srgb8: Srgba<u8> = (foreground_as_f32.over(self_as_f32))
                            .unpremultiply()
                            .into_format();
                        ComparableColor {
                            red: blended_as_srgb8.red,
                            green: blended_as_srgb8.green,
                            blue: blended_as_srgb8.blue,
                            alpha: blended_as_srgb8.alpha,
                        }
                    }
                })
                .collect()
        }
    }

    pub fn as_f32_srgba(&self) -> Srgba<f32> {
        Srgba::<u8>::new(self.red, self.green, self.blue, self.alpha).into_format()
    }

    pub const TRANSPARENT: ComparableColor = rgba(0, 0, 0, 0);
    pub const BLACK: ComparableColor = gray(0);
    pub const RED: ComparableColor = rgb(u8::MAX, 0, 0);
    pub const GREEN: ComparableColor = rgb(0, u8::MAX, 0);
    pub const BLUE: ComparableColor = rgb(0, 0, u8::MAX);
    pub const YELLOW: ComparableColor = rgb(u8::MAX, u8::MAX, 0);
    pub const MAGENTA: ComparableColor = rgb(u8::MAX, 0, u8::MAX);
    pub const CYAN: ComparableColor = rgb(0, u8::MAX, u8::MAX);
    pub const WHITE: ComparableColor = gray(u8::MAX);

    pub const STONE_EXTREME_SHADOW: ComparableColor = gray(0x55);
    pub const STONE_SHADOW: ComparableColor = gray(0x77);
    pub const STONE: ComparableColor = gray(0x88);
    pub const STONE_HIGHLIGHT: ComparableColor = gray(0xaa);
    pub const STONE_EXTREME_HIGHLIGHT: ComparableColor = gray(0xbb);

    pub const DEEPSLATE_SHADOW: ComparableColor = c(0x2f2f3f);
    pub const DEEPSLATE: ComparableColor = ComparableColor::STONE_EXTREME_SHADOW;
    pub const DEEPSLATE_HIGHLIGHT: ComparableColor = ComparableColor::STONE_SHADOW;

    pub const EXTRA_DARK_BIOME_COLORABLE: ComparableColor = ComparableColor::STONE_SHADOW;
    pub const DARK_BIOME_COLORABLE: ComparableColor = ComparableColor::STONE;
    pub const MEDIUM_BIOME_COLORABLE: ComparableColor = gray(0x99);
    pub const LIGHT_BIOME_COLORABLE: ComparableColor = ComparableColor::STONE_EXTREME_HIGHLIGHT;
    pub const EXTRA_LIGHT_BIOME_COLORABLE: ComparableColor = gray(0xcc);

    /// If I'm gonna use a gray any darker than this, I may as well just use
    /// [ComparableColor::BLACK] instead.
    pub const DARKEST_GRAY: ComparableColor = gray(0x22);

    /// If I'm gonna use a gray any lighter than this, I may as well just use
    /// [ComparableColor::WHITE] instead.
    pub const LIGHTEST_GRAY: ComparableColor = gray(0xdd);

    pub const RESERVED_FOR_TRANSPARENCY: ComparableColor = c(0xc0ff3e);

    pub const fn is_gray(&self) -> bool {
        self.alpha == 0 || (self.green == self.red && self.blue == self.red)
    }

    pub const fn is_binary_alpha(&self) -> bool {
        self.alpha == 0 || self.alpha == u8::MAX
    }

    pub const fn abs_diff(&self, other: &ComparableColor) -> u16 {
        if self.alpha == 0 && other.alpha == 0 {
            0
        } else {
            self.red.abs_diff(other.red) as u16
                + self.green.abs_diff(other.green) as u16
                + self.blue.abs_diff(other.blue) as u16
                + self.alpha.abs_diff(other.alpha) as u16
        }
    }

    pub fn bit_depth(&self) -> BitDepth {
        if self.alpha() == 0 {
            BitDepth::One
        } else {
            BIT_DEPTH_FOR_CHANNEL[self.red as usize]
                .max(BIT_DEPTH_FOR_CHANNEL[self.green as usize])
                .max(BIT_DEPTH_FOR_CHANNEL[self.blue as usize])
                .max(BIT_DEPTH_FOR_CHANNEL[self.alpha as usize])
        }
    }
}

impl Mul<f32> for ComparableColor {
    type Output = ComparableColor;

    fn mul(self, rhs: f32) -> Self::Output {
        let out_alpha = f32::from(self.alpha) * rhs;
        ComparableColor {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha: (out_alpha + 0.5) as u8,
        }
    }
}

impl Display for ComparableColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.alpha == 0 {
            write!(f, "transparent")
        } else {
            write!(
                f,
                "#{:02x}{:02x}{:02x}{:02x}",
                self.red, self.green, self.blue, self.alpha
            )
        }
    }
}

impl Debug for ComparableColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl From<Color> for ComparableColor {
    fn from(value: Color) -> Self {
        Self::from(value.to_color_u8())
    }
}

impl From<RGBA8> for ComparableColor {
    fn from(value: RGBA8) -> Self {
        ComparableColor {
            red: value.r,
            green: value.g,
            blue: value.b,
            alpha: value.a,
        }
    }
}

impl From<PremultipliedColor> for ComparableColor {
    fn from(value: PremultipliedColor) -> Self {
        ComparableColor::from(value.demultiply())
    }
}

impl From<ColorU8> for ComparableColor {
    fn from(value: ColorU8) -> Self {
        unsafe {
            transmute(value)
        }
    }
}

impl From<PremultipliedColorU8> for ComparableColor {
    fn from(value: PremultipliedColorU8) -> Self {
        ComparableColor::from(value.demultiply())
    }
}

impl From<ComparableColor> for Color {
    fn from(val: ComparableColor) -> Self {
        Color::from_rgba8(val.red, val.green, val.blue, val.alpha)
    }
}

impl From<ComparableColor> for PremultipliedColor {
    fn from(val: ComparableColor) -> Self {
        let color: Color = val.into();
        color.premultiply()
    }
}

impl From<ComparableColor> for ColorU8 {
    fn from(val: ComparableColor) -> Self {
        unsafe {
            transmute(val)
        }
    }
}

impl From<ComparableColor> for PremultipliedColorU8 {
    fn from(val: ComparableColor) -> Self {
        let color: ColorU8 = val.into();
        color.premultiply()
    }
}

impl PartialEq<Self> for ComparableColor {
    fn eq(&self, other: &Self) -> bool {
        (self.alpha == 0 && other.alpha == 0)
            || (self.red == other.red
                && self.green == other.green
                && self.blue == other.blue
                && self.alpha == other.alpha)
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
    assert_eq!(rgba(0, 0, 0, 0), rgba(u8::MAX, u8::MAX, u8::MAX, 0));
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
fn test_under() {
    use std::iter::once;

    let semi_black = rgba(0, 0, 0, 127);
    assert_eq!(
        ComparableColor::TRANSPARENT.under(once(semi_black)),
        &[semi_black]
    );
    assert_eq!(ComparableColor::WHITE.under(once(semi_black)), &[gray(128)]);
    assert_eq!(
        semi_black.under(once(ComparableColor::WHITE)),
        &[ComparableColor::WHITE]
    );
    assert_eq!(semi_black.under(once(semi_black)), &[rgba(0, 0, 0, 191)]);
}

#[test]
fn test_hash() {
    use std::collections::hash_map::DefaultHasher;
    fn hash(color: ComparableColor) -> u64 {
        let mut hasher = DefaultHasher::new();
        color.hash(&mut hasher);
        hasher.finish()
    }
    let black_hash = hash(ComparableColor::BLACK);
    let red_hash = hash(ComparableColor::RED);
    let green_hash = hash(ComparableColor::GREEN);
    let blue_hash = hash(ComparableColor::BLUE);
    let transparent_hash_1 = hash(rgba(0, 0, 0, 0));
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
    assert_eq!(transparent_hash_1, transparent_hash_2);
}

const fn rgb(r: u8, g: u8, b: u8) -> ComparableColor {
    ComparableColor {
        red: r,
        green: g,
        blue: b,
        alpha: u8::MAX,
    }
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> ComparableColor {
    ComparableColor {
        red: r,
        green: g,
        blue: b,
        alpha: a,
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
        alpha: u8::MAX,
    }
}

#[test]
fn test_c() {
    assert_eq!(
        c(0xc0ffee),
        ComparableColor {
            red: 0xc0,
            green: 0xff,
            blue: 0xee,
            alpha: u8::MAX
        }
    )
}

#[test]
fn test_ord() {
    assert_eq!(
        ComparableColor::TRANSPARENT,
        ComparableColor {
            red: u8::MAX,
            green: u8::MAX,
            blue: u8::MAX,
            alpha: 0
        }
    );
    assert!(
        ComparableColor::TRANSPARENT
            < ComparableColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 1
            }
    );
    assert!(
        ComparableColor {
            red: u8::MAX,
            green: u8::MAX,
            blue: u8::MAX,
            alpha: 1
        } < ComparableColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 2
        }
    );
}
