// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The work of `std.matrix` (spec 2.17, decision 0111) on flat lists of
//! numbers, row by row: the product (`product.rs`), item-by-item
//! arithmetic, sums and transposes. Whole numbers overflow as Kavach's do:
//! the work stops and says so, and the caller panics. Floats never fuse a
//! multiply with an add here, so every machine gives the same bits.

mod product;
pub use product::product;

/// A sum or product of whole numbers that doesn't fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overflow;

/// What `zip` does to two items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

/// A number type a matrix holds.
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

/// `a op b`, item by item; `a` and `b` are as long.
pub fn zip<T: Num>(op: Op, a: &[T], b: &[T]) -> Result<Vec<T>, Overflow> {
    let f = match op {
        Op::Add => T::add,
        Op::Sub => T::sub,
        Op::Mul => T::mul,
    };
    a.iter()
        .zip(b)
        .map(|(x, y)| f(*x, *y).ok_or(Overflow))
        .collect()
}

/// Every item times `x`.
pub fn scale<T: Num>(a: &[T], x: T) -> Result<Vec<T>, Overflow> {
    a.iter().map(|v| v.mul(x).ok_or(Overflow)).collect()
}

/// Every item negated.
pub fn negate<T: Num>(a: &[T]) -> Result<Vec<T>, Overflow> {
    a.iter().map(|v| v.neg().ok_or(Overflow)).collect()
}

/// The `rows` × `cols` matrix `a` with rows and columns swapped.
pub fn transpose<T: Num>(a: &[T], rows: usize, cols: usize) -> Vec<T> {
    let mut out = vec![T::default(); a.len()];
    for i in 0..rows {
        for j in 0..cols {
            out[j * rows + i] = a[i * cols + j];
        }
    }
    out
}

/// What `sums` adds up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum By {
    All,
    Rows,
    Cols,
}

/// The sum of every item (one number), of each row, or of each column,
/// each added in order: row by row, and down each column.
pub fn sums<T: Num>(a: &[T], rows: usize, cols: usize, by: By) -> Result<Vec<T>, Overflow> {
    let add = |s: T, x: &T| s.add(*x).ok_or(Overflow);
    match by {
        By::All => Ok(vec![a.iter().try_fold(T::default(), add)?]),
        By::Rows => (0..rows)
            .map(|i| {
                a[i * cols..(i + 1) * cols]
                    .iter()
                    .try_fold(T::default(), add)
            })
            .collect(),
        By::Cols => {
            let mut out = vec![T::default(); cols];
            for i in 0..rows {
                for j in 0..cols {
                    out[j] = add(out[j], &a[i * cols + j])?;
                }
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests;
