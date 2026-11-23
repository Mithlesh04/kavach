// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whole numbers wider than 64 bits (spec 2.1): `Int<bits: 128>`,
//! `UInt<bits: 256>`, up to 65,535 bits. Their values are `BigInt`s, and
//! each operation checks the result against the type's limits, with the
//! words of the narrower ones. The interpreter and native code both call
//! these, so they say the same.

use num_bigint::{BigInt, BigUint, Sign};

fn one() -> BigInt {
    BigInt::from(1)
}

fn zero() -> BigInt {
    BigInt::from(0)
}

fn negative(n: &BigInt) -> bool {
    n.sign() == Sign::Minus
}

/// A `bits` type wider than 64: its width and whether it has a sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wide {
    pub bits: u16,
    pub signed: bool,
}

impl Wide {
    /// Its smallest and largest values.
    pub fn limits(self) -> (BigInt, BigInt) {
        let n = usize::from(self.bits);
        if self.signed {
            let half = one() << (n - 1);
            (-&half, half - 1)
        } else {
            (zero(), (one() << n) - 1)
        }
    }

    /// Its name, as a program writes it.
    pub fn name(self) -> String {
        let u = if self.signed { "" } else { "U" };
        format!("{u}Int<bits: {}>", self.bits)
    }

    pub fn fits(self, n: &BigInt) -> bool {
        if self.signed {
            // Two's complement in `bits`: the sign bit included.
            n.bits() < u64::from(self.bits)
                || (negative(n) && *n == -(one() << (usize::from(self.bits) - 1)))
        } else {
            !negative(n) && n.bits() <= u64::from(self.bits)
        }
    }

    /// `n` wrapped into the type, as a register of `bits` bits keeps it.
    fn wrap(self, n: &BigInt) -> BigInt {
        let (lo, hi) = self.limits();
        let span = &hi - &lo + 1;
        let mut r = (n - &lo) % &span;
        if negative(&r) {
            r += &span;
        }
        r + lo
    }

    /// `a op b` for `op` in `kavach_kir::WHOLE_OPS` (`neg` and `~` read
    /// only `a`). `Ok(None)` is a `checked_*` that doesn't fit; `Err` is
    /// the panic's message.
    pub fn op(self, op: &str, a: &BigInt, b: &BigInt) -> Result<Option<BigInt>, String> {
        let overflow = |text: String| {
            Err(format!(
                "integer overflow: {text} doesn't fit in {}",
                self.name()
            ))
        };
        let exact = |op: &str| -> Result<Option<BigInt>, String> {
            Ok(match op {
                "+" | "wrapping_add" | "checked_add" => Some(a + b),
                "-" | "wrapping_sub" | "checked_sub" => Some(a - b),
                "*" | "wrapping_mul" | "checked_mul" => Some(a * b),
                "/" | "%" if b.sign() == Sign::NoSign => return Err("division by zero".to_string()),
                "checked_div" if b.sign() == Sign::NoSign => None,
                "/" | "checked_div" => Some(a / b),
                "%" => Some(a % b),
                "**" if negative(b) => {
                    return Err(format!(
                        "{a} ** {b}: a whole number to a negative power isn't whole; use floats"
                    ));
                }
                // Past the width, only 0, 1 and -1 give a result that fits.
                "**" if *a.magnitude() > BigUint::from(1u8) && *b > BigInt::from(self.bits) => None,
                "**" => crate::big_pow(a, b).ok(),
                "&" => Some(a & b),
                "|" => Some(a | b),
                "^" => Some(a ^ b),
                "<<" | ">>" => {
                    let width = BigInt::from(self.bits);
                    if negative(b) || *b >= width {
                        None
                    } else {
                        let s = usize::try_from(b).unwrap_or(0);
                        Some(if op == "<<" { a << s } else { a >> s })
                    }
                }
                _ => None,
            })
        };
        match op {
            "neg" => {
                let n = -a;
                if self.fits(&n) {
                    Ok(Some(n))
                } else {
                    overflow(format!("-({a})"))
                }
            }
            "~" => {
                let n = !a;
                if self.fits(&n) {
                    return Ok(Some(n));
                }
                Ok(Some(n & self.limits().1))
            }
            _ if op.starts_with("wrapping_") => {
                Ok(Some(exact(op)?.map_or(zero(), |n| self.wrap(&n))))
            }
            _ if op.starts_with("checked_") => Ok(exact(op)?.filter(|n| self.fits(n))),
            _ => match exact(op)?.filter(|n| self.fits(n)) {
                Some(n) => Ok(Some(n)),
                None => overflow(format!("{a} {op} {b}")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(n: i128) -> BigInt {
        BigInt::from(n)
    }

    #[test]
    fn limits_and_fits_agree() {
        for (bits, signed) in [
            (65, true),
            (65, false),
            (128, true),
            (128, false),
            (300, true),
        ] {
            let w = Wide { bits, signed };
            let (lo, hi) = w.limits();
            assert!(w.fits(&lo) && w.fits(&hi), "{bits} {signed}");
            assert!(
                !w.fits(&(&lo - 1)) && !w.fits(&(&hi + 1)),
                "{bits} {signed}"
            );
        }
        let u128_max = Wide {
            bits: 128,
            signed: false,
        }
        .limits()
        .1;
        assert_eq!(u128_max, BigInt::from(u128::MAX));
        assert_eq!(
            Wide {
                bits: 128,
                signed: true
            }
            .limits()
            .0,
            big(i128::MIN)
        );
    }

    #[test]
    fn operations_check_the_width() {
        let w = Wide {
            bits: 128,
            signed: false,
        };
        let max = w.limits().1;
        assert_eq!(
            w.op("+", &max, &big(1)),
            Err(format!(
                "integer overflow: {max} + 1 doesn't fit in UInt<bits: 128>"
            ))
        );
        assert_eq!(w.op("wrapping_add", &max, &big(1)), Ok(Some(big(0))));
        assert_eq!(w.op("checked_add", &max, &big(1)), Ok(None));
        assert_eq!(w.op("~", &big(0), &big(0)), Ok(Some(max.clone())));
        assert_eq!(w.op("<<", &big(1), &big(127)), Ok(Some(big(1) << 127)));
        assert!(w.op("<<", &big(1), &big(128)).is_err());
        assert!(w.op("**", &big(2), &big(1000)).is_err());
        assert_eq!(w.op("**", &big(2), &big(100)), Ok(Some(big(1) << 100)));
        let s = Wide {
            bits: 128,
            signed: true,
        };
        assert_eq!(s.op("~", &big(0), &big(0)), Ok(Some(big(-1))));
        assert!(s.op("neg", &big(i128::MIN), &big(0)).is_err());
        assert_eq!(s.op("/", &big(1), &big(0)), Err("division by zero".into()));
    }
}
