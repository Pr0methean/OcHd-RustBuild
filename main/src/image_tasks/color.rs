use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Mul;
use tiny_skia::Color;
use tiny_skia::ColorU8;
use tiny_skia::PremultipliedColor;
use tiny_skia::PremultipliedColorU8;

/// Wrapper around [ColorU8] that implements important missing traits such as [Eq], [Hash], [Copy],
/// [Clone] and [Ord].
#[derive(Eq, Debug, Copy, Clone, Ord, PartialOrd)]
pub struct ComparableColor {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl ComparableColor {
    pub(crate) fn red(&self) -> u8 { return self.red; }
    pub(crate) fn green(&self) -> u8 { return self.green; }
    pub(crate) fn blue(&self) -> u8 { return self.blue; }
    pub(crate) fn alpha(&self) -> u8 { return self.alpha; }
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
        write!(f, "#{:02x}{:02x}{:02x}{:02x}", self.red, self.green, self.blue, self.alpha)
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
