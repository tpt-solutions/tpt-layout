//! Fixed-point length type. All layout math runs on [`Px`]; floats only appear at the API boundary.
//!
//! Rounding rule everywhere: halves round away from zero. Arithmetic saturates instead of wrapping.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Sub-pixel units per pixel (1/64 px).
pub const PX_SCALE: i32 = 64;

/// A length in 1/64 px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Px(pub i32);

impl Px {
    pub const ZERO: Px = Px(0);
    pub const MAX: Px = Px(i32::MAX);

    /// Whole pixels.
    pub const fn from_px(px: i32) -> Px {
        Px(px.saturating_mul(PX_SCALE))
    }

    /// Converts float pixels, rounding half away from zero. NaN becomes zero; out-of-range
    /// values saturate.
    pub fn from_f32(px: f32) -> Px {
        Px(round_half_away(px * PX_SCALE as f32))
    }

    /// Converts to float pixels. Exact for every value that fits f32's 24-bit mantissa.
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / PX_SCALE as f32
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Converts typographic points (given in hundredths of a point) at `dpi`. Points are 1/72 in.
    pub fn from_points(hundredths: i32, dpi: u32) -> Px {
        Px(hundredths).mul_ratio(dpi as i64 * PX_SCALE as i64, 7200)
    }

    /// Computes `self * num / den` with 64-bit intermediates, rounding half away from zero.
    /// `den` must be non-zero.
    pub fn mul_ratio(self, num: i64, den: i64) -> Px {
        debug_assert!(den != 0, "mul_ratio: zero denominator");
        let n = (self.0 as i64).saturating_mul(num);
        Px(saturate_i32(div_round_i64(n, den)))
    }

    /// Applies a percentage expressed in hundredths of a percent (`5000` = 50%).
    pub fn percent(self, hundredths: i32) -> Px {
        self.mul_ratio(hundredths as i64, 10_000)
    }

    /// Clamps to `[min, max]` with CSS precedence: `min` wins when the bounds conflict.
    pub fn clamp_to(self, min: Px, max: Option<Px>) -> Px {
        let bounded = match max {
            Some(max) => self.min(max),
            None => self,
        };
        bounded.max(min)
    }
}

impl Add for Px {
    type Output = Px;
    fn add(self, rhs: Px) -> Px {
        Px(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Px {
    type Output = Px;
    fn sub(self, rhs: Px) -> Px {
        Px(self.0.saturating_sub(rhs.0))
    }
}

impl Neg for Px {
    type Output = Px;
    fn neg(self) -> Px {
        Px(self.0.saturating_neg())
    }
}

impl AddAssign for Px {
    fn add_assign(&mut self, rhs: Px) {
        *self = *self + rhs;
    }
}

impl SubAssign for Px {
    fn sub_assign(&mut self, rhs: Px) {
        *self = *self - rhs;
    }
}

/// Multiplies by a whole number (e.g. a child count or a span).
impl Mul<i32> for Px {
    type Output = Px;
    fn mul(self, rhs: i32) -> Px {
        Px(self.0.saturating_mul(rhs))
    }
}

/// Divides by a whole number, rounding half away from zero. Panics on a zero divisor, like
/// integer division.
impl Div<i32> for Px {
    type Output = Px;
    fn div(self, rhs: i32) -> Px {
        Px(saturate_i32(div_round_i64(self.0 as i64, rhs as i64)))
    }
}

/// Integer division rounding half away from zero. `d` must be non-zero.
fn div_round_i64(n: i64, d: i64) -> i64 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    let half = d / 2;
    if n >= 0 {
        (n + half) / d
    } else {
        (n - half) / d
    }
}

fn saturate_i32(v: i64) -> i32 {
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Rounds half away from zero. NaN maps to 0; `as` casts saturate at the i32 bounds.
pub(crate) fn round_half_away(v: f32) -> i32 {
    if v.is_nan() {
        return 0;
    }
    let shifted = if v >= 0.0 { v + 0.5 } else { v - 0.5 };
    shifted as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_f32_rounds_half_away_from_zero() {
        assert_eq!(Px::from_f32(1.0), Px(64));
        assert_eq!(Px::from_f32(0.5 / 64.0), Px(1));
        assert_eq!(Px::from_f32(-0.5 / 64.0), Px(-1));
        assert_eq!(Px::from_f32(f32::NAN), Px::ZERO);
        assert_eq!(Px::from_f32(f32::INFINITY), Px::MAX);
    }

    #[test]
    fn to_f32_is_exact_for_fixed_values() {
        assert_eq!(Px(96).to_f32(), 1.5);
        assert_eq!(Px::from_px(-3).to_f32(), -3.0);
    }

    #[test]
    fn division_rounds_half_away() {
        assert_eq!(Px(5) / 2, Px(3));
        assert_eq!(Px(-5) / 2, Px(-3));
        assert_eq!(Px(4) / 2, Px(2));
    }

    #[test]
    fn percent_and_ratio() {
        assert_eq!(Px::from_px(400).percent(5000), Px::from_px(200));
        assert_eq!(Px::from_px(10).mul_ratio(1, 3), Px(213)); // 640/3 = 213.33 -> 213
    }

    #[test]
    fn arithmetic_saturates() {
        assert_eq!(Px::MAX + Px(1), Px::MAX);
        assert_eq!(Px(i32::MIN) - Px(1), Px(i32::MIN));
        assert_eq!(Px::MAX * 2, Px::MAX);
    }

    #[test]
    fn clamp_gives_min_precedence() {
        let min = Px::from_px(10);
        assert_eq!(Px::from_px(5).clamp_to(min, None), min);
        assert_eq!(
            Px::from_px(50).clamp_to(min, Some(Px::from_px(20))),
            Px::from_px(20)
        );
        // min > max: min wins, as in CSS.
        assert_eq!(Px::from_px(15).clamp_to(min, Some(Px::from_px(5))), min);
    }
}
