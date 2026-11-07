//! Exact base-10 numbers for money (spec 2.1): `0.1 + 0.2 == 0.3`.

use std::cmp::Ordering;
use std::fmt;

/// `mantissa × 10^-scale`. `499.00` is (49900, 2) and keeps printing as
/// `499.00` (spec 1.11).
#[derive(Clone, Copy, Debug)]
pub struct Decimal {
    pub mantissa: i128,
    pub scale: u32,
}

/// Digits kept after the point when a division doesn't end.
pub(crate) const DIVISION_SCALE: u32 = 18;

// `add`, `sub`… return `None` on overflow, so they are not the `std::ops` traits.
#[allow(clippy::should_implement_trait)]
impl Decimal {
    pub fn from_int(n: i128) -> Decimal {
        Decimal {
            mantissa: n,
            scale: 0,
        }
    }

    /// Parses literal text such as `499.00`, `1_000` or `2.5e-3`.
    pub fn parse(text: &str) -> Option<Decimal> {
        let text = text.replace('_', "");
        let (number, exponent) = match text.split_once(['e', 'E']) {
            Some((n, e)) => (n.to_string(), e.parse::<i32>().ok()?),
            None => (text, 0),
        };
        let (int, frac) = number.split_once('.').unwrap_or((&number, ""));
        let mantissa: i128 = format!("{int}{frac}").parse().ok()?;
        let scale = frac.len() as i32 - exponent;
        Some(if scale >= 0 {
            Decimal {
                mantissa,
                scale: scale as u32,
            }
        } else {
            Decimal {
                mantissa: mantissa.checked_mul(10i128.checked_pow((-scale) as u32)?)?,
                scale: 0,
            }
        })
    }

    fn rescale(self, scale: u32) -> Option<i128> {
        self.mantissa
            .checked_mul(10i128.checked_pow(scale - self.scale)?)
    }

    fn aligned(self, other: Decimal) -> Option<(i128, i128, u32)> {
        let scale = self.scale.max(other.scale);
        Some((self.rescale(scale)?, other.rescale(scale)?, scale))
    }

    pub fn add(self, other: Decimal) -> Option<Decimal> {
        let (a, b, scale) = self.aligned(other)?;
        Some(Decimal {
            mantissa: a.checked_add(b)?,
            scale,
        })
    }

    pub fn sub(self, other: Decimal) -> Option<Decimal> {
        let (a, b, scale) = self.aligned(other)?;
        Some(Decimal {
            mantissa: a.checked_sub(b)?,
            scale,
        })
    }

    pub fn mul(self, other: Decimal) -> Option<Decimal> {
        Some(Decimal {
            mantissa: self.mantissa.checked_mul(other.mantissa)?,
            scale: self.scale + other.scale,
        })
    }

    /// `None` on division by zero or overflow.
    pub fn div(self, other: Decimal) -> Option<Decimal> {
        if other.mantissa == 0 {
            return None;
        }
        let scale = self.scale.max(other.scale).max(DIVISION_SCALE);
        let a = self.rescale(scale + other.scale)?;
        let q = a / other.mantissa;
        Some(Decimal { mantissa: q, scale }.trimmed(self.scale.max(other.scale)))
    }

    /// Removes trailing zeros beyond `keep` digits after the point.
    fn trimmed(mut self, keep: u32) -> Decimal {
        while self.scale > keep && self.mantissa % 10 == 0 {
            self.mantissa /= 10;
            self.scale -= 1;
        }
        self
    }

    /// Rounded to `places` digits after the point, as
    /// [`AnyDecimal::round`](crate::AnyDecimal::round). It always fits:
    /// rounding drops digits, so the result is never larger.
    pub fn round(self, places: u32, even: bool) -> Decimal {
        if self.scale <= places {
            return self;
        }
        // Dropping more digits than an `i128` holds leaves less than half
        // of the last place kept: 0.
        let Some(drop) = 10i128.checked_pow(self.scale - places) else {
            return Decimal {
                mantissa: 0,
                scale: places,
            };
        };
        let (q, r) = (self.mantissa / drop, self.mantissa % drop);
        let up = match (r.unsigned_abs() * 2).cmp(&drop.unsigned_abs()) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => !even || q % 2 != 0,
        };
        // `q` is at most a tenth of the mantissa, so one more fits.
        let q = match (up, self.mantissa < 0) {
            (true, true) => q - 1,
            (true, false) => q + 1,
            (false, _) => q,
        };
        Decimal {
            mantissa: q,
            scale: places,
        }
    }

    pub fn neg(self) -> Decimal {
        Decimal {
            mantissa: -self.mantissa,
            scale: self.scale,
        }
    }

    pub fn to_f64(self) -> f64 {
        self.mantissa as f64 / 10f64.powi(self.scale as i32)
    }

    /// The whole-number part, rounded toward zero.
    pub fn trunc(self) -> i128 {
        self.mantissa / 10i128.pow(self.scale)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (a, b, _) = self.aligned(*other)?;
        Some(a.cmp(&b))
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.mantissa < 0 { "-" } else { "" };
        let digits = self.mantissa.unsigned_abs().to_string();
        if self.scale == 0 {
            return write!(f, "{sign}{digits}");
        }
        let digits = format!("{digits:0>width$}", width = self.scale as usize + 1);
        let (int, frac) = digits.split_at(digits.len() - self.scale as usize);
        write!(f, "{sign}{int}.{frac}")
    }
}
