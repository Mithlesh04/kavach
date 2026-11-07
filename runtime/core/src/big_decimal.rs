//! `Decimal` values too big or too precise for its 128-bit form (spec
//! 2.1, decision 0042): `num-bigint` digits with a scale. A result that
//! fits the fast form goes back to it, so each value has one form.

use crate::decimal::{DIVISION_SCALE, Decimal};
use num_bigint::{BigInt, Sign};
use std::cmp::Ordering;
use std::fmt;

/// `mantissa × 10^-scale`, like [`Decimal`], with no size limit.
#[derive(Clone, Debug)]
pub struct BigDecimal {
    pub mantissa: BigInt,
    pub scale: u32,
}

/// A decimal in either form: what arithmetic takes and gives.
#[derive(Clone, Debug)]
pub enum AnyDecimal {
    Small(Decimal),
    Big(BigDecimal),
}

fn ten_to(n: u32) -> BigInt {
    BigInt::from(10).pow(n)
}

impl BigDecimal {
    fn of(d: Decimal) -> BigDecimal {
        BigDecimal {
            mantissa: BigInt::from(d.mantissa),
            scale: d.scale,
        }
    }

    /// Parses literal text as [`Decimal::parse`] does, with any number of
    /// digits.
    pub fn parse(text: &str) -> Option<BigDecimal> {
        let text = text.replace('_', "");
        let (number, exponent) = match text.split_once(['e', 'E']) {
            Some((n, e)) => (n.to_string(), e.parse::<i32>().ok()?),
            None => (text, 0),
        };
        let (int, frac) = number.split_once('.').unwrap_or((&number, ""));
        let mantissa: BigInt = format!("{int}{frac}").parse().ok()?;
        let scale = frac.len() as i32 - exponent;
        Some(if scale >= 0 {
            BigDecimal {
                mantissa,
                scale: scale as u32,
            }
        } else {
            BigDecimal {
                mantissa: mantissa * ten_to((-scale) as u32),
                scale: 0,
            }
        })
    }

    fn rescaled(&self, scale: u32) -> BigInt {
        &self.mantissa * ten_to(scale - self.scale)
    }

    fn aligned(&self, other: &BigDecimal) -> (BigInt, BigInt, u32) {
        let scale = self.scale.max(other.scale);
        (self.rescaled(scale), other.rescaled(scale), scale)
    }

    /// The fast form when the value fits it.
    fn settled(self) -> AnyDecimal {
        match i128::try_from(&self.mantissa) {
            Ok(mantissa) => AnyDecimal::Small(Decimal {
                mantissa,
                scale: self.scale,
            }),
            Err(_) => AnyDecimal::Big(self),
        }
    }

    fn compare(&self, other: &BigDecimal) -> Ordering {
        let (a, b, _) = self.aligned(other);
        a.cmp(&b)
    }
}

impl AnyDecimal {
    /// Literal text in whichever form holds it.
    pub fn parse(text: &str) -> Option<AnyDecimal> {
        match Decimal::parse(text) {
            Some(d) => Some(AnyDecimal::Small(d)),
            None => BigDecimal::parse(text).map(BigDecimal::settled),
        }
    }

    fn big(&self) -> BigDecimal {
        match self {
            AnyDecimal::Small(d) => BigDecimal::of(*d),
            AnyDecimal::Big(b) => b.clone(),
        }
    }

    fn exact(&self, other: &AnyDecimal, op: fn(&BigInt, &BigInt) -> BigInt) -> AnyDecimal {
        let (a, b, scale) = self.big().aligned(&other.big());
        BigDecimal {
            mantissa: op(&a, &b),
            scale,
        }
        .settled()
    }

    pub fn add(&self, other: &AnyDecimal) -> AnyDecimal {
        if let (AnyDecimal::Small(x), AnyDecimal::Small(y)) = (self, other)
            && let Some(d) = x.add(*y)
        {
            return AnyDecimal::Small(d);
        }
        self.exact(other, |a, b| a + b)
    }

    pub fn sub(&self, other: &AnyDecimal) -> AnyDecimal {
        if let (AnyDecimal::Small(x), AnyDecimal::Small(y)) = (self, other)
            && let Some(d) = x.sub(*y)
        {
            return AnyDecimal::Small(d);
        }
        self.exact(other, |a, b| a - b)
    }

    pub fn mul(&self, other: &AnyDecimal) -> AnyDecimal {
        if let (AnyDecimal::Small(x), AnyDecimal::Small(y)) = (self, other)
            && let Some(d) = x.mul(*y)
        {
            return AnyDecimal::Small(d);
        }
        let (x, y) = (self.big(), other.big());
        BigDecimal {
            mantissa: x.mantissa * y.mantissa,
            scale: x.scale + y.scale,
        }
        .settled()
    }

    /// `None` on division by zero.
    pub fn div(&self, other: &AnyDecimal) -> Option<AnyDecimal> {
        if self.is_zero_divisor(other) {
            return None;
        }
        if let (AnyDecimal::Small(x), AnyDecimal::Small(y)) = (self, other)
            && let Some(d) = x.div(*y)
        {
            return Some(AnyDecimal::Small(d));
        }
        // As `Decimal::div`: 18 places when it doesn't end, then trailing
        // zeros dropped down to the scale of the operands.
        let (x, y) = (self.big(), other.big());
        let keep = x.scale.max(y.scale);
        let mut scale = keep.max(DIVISION_SCALE);
        let mut q = x.rescaled(scale + y.scale) / &y.mantissa;
        let ten = BigInt::from(10);
        while scale > keep && (&q % &ten).sign() == Sign::NoSign {
            q /= &ten;
            scale -= 1;
        }
        Some(BigDecimal { mantissa: q, scale }.settled())
    }

    fn is_zero_divisor(&self, other: &AnyDecimal) -> bool {
        let _ = self;
        match other {
            AnyDecimal::Small(d) => d.mantissa == 0,
            AnyDecimal::Big(b) => b.mantissa.sign() == Sign::NoSign,
        }
    }

    pub fn neg(&self) -> AnyDecimal {
        match self {
            AnyDecimal::Small(d) if d.mantissa != i128::MIN => AnyDecimal::Small(d.neg()),
            _ => {
                let b = self.big();
                BigDecimal {
                    mantissa: -b.mantissa,
                    scale: b.scale,
                }
                .settled()
            }
        }
    }

    pub fn compare(&self, other: &AnyDecimal) -> Ordering {
        match (self, other) {
            (AnyDecimal::Small(x), AnyDecimal::Small(y)) => match x.partial_cmp(y) {
                Some(o) => o,
                None => self.big().compare(&other.big()),
            },
            _ => self.big().compare(&other.big()),
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            AnyDecimal::Small(d) => d.to_f64(),
            AnyDecimal::Big(_) => self.to_string().parse().unwrap_or(f64::INFINITY),
        }
    }

    /// Rounded to `places` digits after the point: a half away from zero,
    /// or with `even` to the even digit (spec 14.2, decision 0051). One
    /// with fewer places is given back as it is.
    pub fn round(&self, places: u32, even: bool) -> AnyDecimal {
        if let AnyDecimal::Small(d) = self {
            return AnyDecimal::Small(d.round(places, even));
        }
        let b = self.big();
        if b.scale <= places {
            return self.clone();
        }
        let drop = ten_to(b.scale - places);
        let (q, r) = (&b.mantissa / &drop, &b.mantissa % &drop);
        let twice: BigInt = if r.sign() == Sign::Minus { -r } else { r } * 2;
        let up = match twice.cmp(&drop) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => !even || (&q % BigInt::from(2)).sign() != Sign::NoSign,
        };
        let q = match (up, b.mantissa.sign()) {
            (true, Sign::Minus) => q - 1,
            (true, _) => q + 1,
            (false, _) => q,
        };
        BigDecimal {
            mantissa: q,
            scale: places,
        }
        .settled()
    }

    /// The whole-number part, rounded toward zero.
    pub fn trunc(&self) -> BigInt {
        let b = self.big();
        b.mantissa / ten_to(b.scale)
    }
}

impl fmt::Display for BigDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.mantissa.sign() == Sign::Minus {
            "-"
        } else {
            ""
        };
        let digits = self.mantissa.magnitude().to_string();
        if self.scale == 0 {
            return write!(f, "{sign}{digits}");
        }
        let digits = format!("{digits:0>width$}", width = self.scale as usize + 1);
        let (int, frac) = digits.split_at(digits.len() - self.scale as usize);
        write!(f, "{sign}{int}.{frac}")
    }
}

impl fmt::Display for AnyDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyDecimal::Small(d) => d.fmt(f),
            AnyDecimal::Big(b) => b.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> AnyDecimal {
        AnyDecimal::parse(text).unwrap()
    }

    #[test]
    fn big_values_stay_exact_and_small_ones_stay_small() {
        let cases = [
            (
                "99999999999999999999999999999999999999.99",
                "0.000000000000000000000000000001",
                "99999999999999999999999999999999999999.990000000000000000000000000001",
            ),
            (
                "170141183460469231731687303715884105727",
                "1",
                "170141183460469231731687303715884105728",
            ),
            ("0.1", "0.2", "0.3"),
        ];
        for (a, b, sum) in cases {
            assert_eq!(d(a).add(&d(b)).to_string(), sum);
        }
        let back = d("170141183460469231731687303715884105728").sub(&d("1"));
        assert!(matches!(back, AnyDecimal::Small(_)));
        assert_eq!(
            d("1e40").mul(&d("1e40")).to_string(),
            format!("1{}", "0".repeat(80))
        );
        assert_eq!(
            d("1e40").div(&d("3")).unwrap().trunc().to_string(),
            format!("3{}", "3".repeat(39))
        );
        assert_eq!(
            d("1").div(&d("3")).unwrap().to_string(),
            "0.333333333333333333"
        );
        assert!(d("1e40").div(&d("0")).is_none());
        assert_eq!(d("1e40").compare(&d("1e39")), Ordering::Greater);
        assert_eq!(d("-1e40").neg().to_string(), format!("1{}", "0".repeat(40)));
    }

    #[test]
    fn rounding_half_away_from_zero_and_half_to_even() {
        // (value, places, round, round_even)
        let cases = [
            ("2.345", 2, "2.35", "2.34"),
            ("2.355", 2, "2.36", "2.36"),
            ("-2.345", 2, "-2.35", "-2.34"),
            ("2.344", 2, "2.34", "2.34"),
            ("0.5", 0, "1", "0"),
            ("1.5", 0, "2", "2"),
            ("-0.5", 0, "-1", "0"),
            ("2.5", 2, "2.5", "2.5"),
            (
                "123456789012345678901234567890.125",
                2,
                "123456789012345678901234567890.13",
                "123456789012345678901234567890.12",
            ),
        ];
        for (value, places, up, even) in cases {
            assert_eq!(
                d(value).round(places, false).to_string(),
                up,
                "{value}.round({places})"
            );
            assert_eq!(
                d(value).round(places, true).to_string(),
                even,
                "{value}.round_even({places})"
            );
        }
    }
}
