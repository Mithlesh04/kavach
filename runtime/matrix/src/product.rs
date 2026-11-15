//! The matrix product (spec 2.17, decision 0111). By default each item
//! adds its products in order, `k` from 0 up, a multiply then an add: the
//! inner loop runs along a row of the result, so the compiler turns it into
//! vector instructions without changing any item's order, and every
//! machine gives the same bits. With `fast`, `Float` and `Float32` go
//! through `gemm`, which may add in another order and fuse. A big product
//! is cut into bands of rows, one thread each.

use super::{Num, Overflow};

/// Products with at least this many multiply-adds use every core.
const SPREAD: usize = 1 << 20;

/// `a @ b`: `a` is `n` × `m`, `b` is `m` × `p`, the result `n` × `p`, all
/// row by row.
pub fn product<T: Num>(
    a: &[T],
    b: &[T],
    (n, m, p): (usize, usize, usize),
    fast: bool,
) -> Result<Vec<T>, Overflow> {
    let mut out = vec![T::default(); n * p];
    if out.is_empty() {
        return Ok(out);
    }
    let cores = std::thread::available_parallelism().map_or(1, |c| c.get());
    let threads = if n * m * p >= SPREAD { cores.min(n) } else { 1 };
    let band = n.div_ceil(threads);
    let work = |rows: &mut [T], first: usize| {
        if fast && T::FLOAT {
            fast_band(a, b, rows, first, (m, p));
            Ok(())
        } else {
            exact_band(a, b, rows, first, (m, p))
        }
    };
    if threads == 1 {
        work(&mut out, 0)?;
        return Ok(out);
    }
    std::thread::scope(|s| {
        let bands: Vec<_> = (out.chunks_mut(band * p).enumerate())
            .map(|(t, rows)| s.spawn(move || work(rows, t * band)))
            .collect();
        (bands.into_iter()).try_for_each(|b| b.join().expect("a band of the product"))
    })?;
    Ok(out)
}

/// The rows from `first` of the product, in order.
fn exact_band<T: Num>(
    a: &[T],
    b: &[T],
    rows: &mut [T],
    first: usize,
    (m, p): (usize, usize),
) -> Result<(), Overflow> {
    for (r, row) in rows.chunks_mut(p).enumerate() {
        let i = first + r;
        for k in 0..m {
            let x = a[i * m + k];
            for (c, y) in row.iter_mut().zip(&b[k * p..(k + 1) * p]) {
                *c = c.add(x.mul(*y).ok_or(Overflow)?).ok_or(Overflow)?;
            }
        }
    }
    Ok(())
}

/// The same rows through `gemm`, for `Float` and `Float32`.
#[allow(unsafe_code)]
fn fast_band<T: Num>(a: &[T], b: &[T], rows: &mut [T], first: usize, (m, p): (usize, usize)) {
    let count = rows.len() / p;
    let lhs = &a[first * m..(first + count) * m];
    // SAFETY: `lhs` is `count` × `m`, `b` is `m` × `p` and `rows` is
    // `count` × `p`, each row by row: a column stride of 1 and a row
    // stride of its width, so `gemm` reads and writes only their items.
    // `T` is `f32` or `f64` (`T::FLOAT`), which `gemm` supports. With
    // `read_dst` false, `rows` becomes `1 × lhs × b`.
    unsafe {
        gemm::gemm(
            count,
            p,
            m,
            rows.as_mut_ptr(),
            1,
            p as isize,
            false,
            lhs.as_ptr(),
            1,
            m as isize,
            b.as_ptr(),
            1,
            p as isize,
            T::default(),
            T::one(),
            false,
            false,
            false,
            gemm::Parallelism::None,
        );
    }
}
