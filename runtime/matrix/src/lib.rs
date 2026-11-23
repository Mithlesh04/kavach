// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The matrix product of `std.tensor`'s `a @ b` (spec 14.33, decisions
//! 0111 and 0136) on flat lists of numbers, row by row (`product.rs`).
//! Whole numbers overflow as Kavach's do: the work stops and says so, and
//! the caller panics. Floats never fuse a multiply with an add here, so
//! every machine gives the same bits.

mod product;
pub use product::product;

/// A sum or product of whole numbers that doesn't fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overflow;

/// A number type the product works on.
pub trait Num: Copy + Default + PartialEq + Send + Sync + 'static {
    fn add(self, other: Self) -> Option<Self>;
    fn sub(self, other: Self) -> Option<Self>;
    fn mul(self, other: Self) -> Option<Self>;
    fn neg(self) -> Option<Self>;
    fn one() -> Self;
    /// `Float` and `Float32`: `gemm` can work them out with `fast`.
    const FLOAT: bool = false;
}

macro_rules! whole {
    ($($t:ty),*) => {$(
        impl Num for $t {
            fn add(self, other: Self) -> Option<Self> { self.checked_add(other) }
            fn sub(self, other: Self) -> Option<Self> { self.checked_sub(other) }
            fn mul(self, other: Self) -> Option<Self> { self.checked_mul(other) }
            fn neg(self) -> Option<Self> { (0 as $t).checked_sub(self) }
            fn one() -> Self { 1 }
        }
    )*};
}
whole!(i8, i16, i32, i64, u8, u16, u32, u64);

macro_rules! float {
    ($($t:ty),*) => {$(
        impl Num for $t {
            fn add(self, other: Self) -> Option<Self> { Some(self + other) }
            fn sub(self, other: Self) -> Option<Self> { Some(self - other) }
            fn mul(self, other: Self) -> Option<Self> { Some(self * other) }
            fn neg(self) -> Option<Self> { Some(-self) }
            fn one() -> Self { 1.0 }
            const FLOAT: bool = true;
        }
    )*};
}
float!(f32, f64);

#[cfg(test)]
mod tests;
