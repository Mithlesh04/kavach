//! C functions (spec 10.7) and raw memory (6.4): native code calls them,
//! as `kavach run --native` and as an executable from `kavach build`; the
//! interpreter stops at the first call and says to run the program
//! natively.

use std::process::Command;

const PROGRAM: &str = "extern c \"c\" {
    fn llabs(n: Int) -> Int
}

extern c \"m\" {
    fn fabs(x: Float) -> Float
}

fn magnitude(n: Int) -> Int {
    unsafe { llabs(n) }
}

fn main() {
    print(magnitude(-42))
    print(unsafe { fabs(-2.5) })
}
";

/// Text and pointers going to C and back, and memory read and written
/// through a `Ptr` of each size.
const POINTERS: &str = "extern c \"c\" {
    fn strlen(s: String) -> UInt64
    fn memset(p: Ptr<UInt8>, value: Int32, n: UInt64) -> Ptr<UInt8>
    fn getenv(name: String) -> Ptr<UInt8>
}

fn c_length(s: String) -> UInt64 {
    unsafe { strlen(s) }
}

fn variable(name: String) -> String? {
    unsafe { getenv(name).text() }
}

fn main() {
    print(c_length(\"hello\"))
    print(c_length(\"नमस्ते\"))
    print(variable(\"KAVACH_C_TEXT\") ?? \"none\")
    print(variable(\"KAVACH_NOT_SET\") ?? \"none\")
    unsafe {
        const bytes: Ptr<UInt8> = alloc(4)
        const _ = memset(bytes, 7, 3)
        print(\"{bytes.read(0)} {bytes.read(2)} {bytes.read(3)}\")
        free(bytes)
        const xs: Ptr<Int> = alloc(3)
        xs.write(1, -5)
        xs.write(2, 9_000_000_000)
        print(xs.read(0) + xs.read(1) + xs.read(2))
        free(xs)
        const fs: Ptr<Float> = alloc(2)
        fs.write(1, 2.5)
        print(fs.read(1) * 2.0)
        free(fs)
        const bs: Ptr<Bool> = alloc(2)
        bs.write(1, true)
        print(\"{bs.read(0)} {bs.read(1)}\")
        free(bs)
        const hs: Ptr<Int16> = alloc(2)
        hs.write(1, -300)
        print(hs.read(1))
        free(hs)
    }
}
";

/// A Kavach function C calls back: `qsort` with a comparison.
const CALLBACKS: &str = "extern c \"c\" {
    fn qsort(items: Ptr<Int32>, count: UInt64, size: UInt64, compare: fn(Ptr<Int32>, Ptr<Int32>) -> Int32)
}

fn by_value(a: Ptr<Int32>, b: Ptr<Int32>) -> Int32 {
    unsafe { a.read(0) - b.read(0) }
}

fn main() {
    unsafe {
        const xs: Ptr<Int32> = alloc(5)
        xs.write(0, 42)
        xs.write(1, -7)
        xs.write(2, 13)
        xs.write(3, 0)
        xs.write(4, 99)
        qsort(xs, 5, 4, by_value)
        print(\"{xs.read(0)} {xs.read(1)} {xs.read(2)} {xs.read(3)} {xs.read(4)}\")
        free(xs)
    }
}
";

/// Records in C memory: sorted by C, copied byte for byte by C (so the
/// layout is C's), read back, and checked against their invariants.
const RECORDS: &str = "type Point {
    x: Int32
    y: Int32
}

// As C's struct { uint8_t a; int64_t b; bool c; int16_t d; }: 24 bytes,
// with `b` at 8, `c` at 16 and `d` at 18.
type Mixed {
    a: UInt8
    b: Int
    c: Bool
    d: Int16
}

type Positive {
    n: Int32
    invariant n > 0
}

extern c \"c\" {
    fn qsort(items: Ptr<Point>, count: UInt64, size: UInt64, compare: fn(Ptr<Point>, Ptr<Point>) -> Int32)
    fn memcpy(to: Ptr<UInt8>, from: Ptr<Mixed>, n: UInt64) -> Ptr<UInt8>
}

fn by_x(a: Ptr<Point>, b: Ptr<Point>) -> Int32 {
    unsafe { a.read(0).x - b.read(0).x }
}

fn main() {
    unsafe {
        const ps: Ptr<Point> = alloc(3)
        ps.write(0, Point(x: 5, y: 50))
        ps.write(1, Point(x: -1, y: 10))
        ps.write(2, Point(x: 3, y: 30))
        qsort(ps, 3, 8, by_x)
        print(\"{ps.read(0).y} {ps.read(1).y} {ps.read(2).y}\")
        free(ps)
        const m: Ptr<Mixed> = alloc(1)
        m.write(0, Mixed(a: 7, b: 1234, c: true, d: -2))
        const bytes: Ptr<UInt8> = alloc(24)
        const _ = memcpy(bytes, m, 24)
        print(\"{bytes.read(0)} {bytes.read(8)} {bytes.read(9)} {bytes.read(16)} {bytes.read(18)} {bytes.read(19)}\")
        const back = m.read(0)
        print(\"{back.a} {back.b} {back.c} {back.d}\")
        free(bytes)
        free(m)
        const zero: Ptr<Positive> = alloc(1)
        print(zero.read(0).n)
    }
}
";

fn kavach() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
}

/// Runs `program` (in a package that allows `unsafe`) as native code and
/// as a built executable: each must print `expected`, with no leaks.
/// Gives the folder, for more runs.
fn native(name: &str, program: &str, expected: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kavach-c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let toml = format!("[package]\nname = \"{name}\"\nunsafe = true\n");
    std::fs::write(dir.join("kavach.toml"), toml).unwrap();
    let file = dir.join("src/main.kav");
    std::fs::write(&file, program).unwrap();
    let jit = kavach()
        .env("KAVACH_CHECK_LEAKS", "1")
        .env("KAVACH_C_TEXT", "from C")
        .args(["run", "--native"])
        .arg(&file)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&jit.stderr);
    assert_eq!(String::from_utf8_lossy(&jit.stdout), expected, "{said}");
    let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let built = kavach()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .output();
    let built = built.unwrap();
    let said = String::from_utf8_lossy(&built.stderr);
    assert!(built.status.success(), "{said}");
    let ran = Command::new(&exe)
        .env("KAVACH_CHECK_LEAKS", "1")
        .env("KAVACH_C_TEXT", "from C")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&ran.stderr);
    assert_eq!(String::from_utf8_lossy(&ran.stdout), expected, "{said}");
    dir
}

#[test]
fn native_code_calls_c_functions() {
    let dir = native("c", PROGRAM, "42\n2.5\n");
    let file = dir.join("src/main.kav");
    let interpreted = kavach().arg("run").arg(&file).output().unwrap();
    // Generated tests skip C functions and what reaches them, which the
    // interpreter can't run: no ✗ that isn't a bug.
    let tested = kavach().arg("test").arg(&file).output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(interpreted.status.code(), Some(101));
    let said = String::from_utf8_lossy(&interpreted.stderr);
    assert!(said.contains("`llabs` is a C function"), "{said}");
    assert!(said.contains("kavach run --native"), "{said}");
    let report = String::from_utf8_lossy(&tested.stdout);
    for skipped in [
        "not generated: llabs: a C function, which runs only in native code (10.7)",
        "not generated: magnitude: reaches C, which runs only in native code (10.7)",
    ] {
        assert!(report.contains(skipped), "{report}");
    }
    assert!(!report.lines().any(|l| l.starts_with('✗')), "{report}");
}

#[test]
fn text_and_pointers_go_to_c() {
    let expected = "5\n18\nfrom C\nnone\n7 7 0\n8999999995\n5.0\nfalse true\n-300\n";
    let dir = native("ptr", POINTERS, expected);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn c_calls_kavach_back() {
    let dir = native("cb", CALLBACKS, "-7 0 13 42 99\n");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn records_are_laid_out_as_c_structs() {
    let expected = "10 30 50\n7 210 4 1 254 255\n7 1234 true -2\n";
    // The last line reads a record that breaks its invariant: a panic.
    let dir = native("rec", RECORDS, expected);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// C's pointer syntax (spec 6.4): `*p`, `*p = v`, `p[i]`, `p[i] = v`,
/// `p + n`, `p - n`, on numbers and on records laid out as C structs.
const SYNTAX: &str = "type Point {
    x: Int32
    y: Int32
}

fn main() {
    unsafe {
        const p: Ptr<Int32> = alloc(3)
        *p = 7
        p[1] = 8
        const q = p + 2
        *q = *p + p[1]
        print(p[2])
        *q += 1
        print(*q)
        const back = q - 1
        print(*back)
        free(p)

        const ps: Ptr<Point> = alloc(2)
        ps[1] = Point(x: 3, y: 4)
        *ps = Point(x: 1, y: 2)
        print(ps[1].y + (*ps).x)
        const second = ps + 1
        print((*second).x)
        free(ps)
    }
}
";

#[test]
fn pointer_syntax_reads_and_writes() {
    let dir = native("syntax", SYNTAX, "15\n16\n8\n5\n3\n");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `Bytes` to C as `const uint8_t*` (spec 10.7), `p.bytes(n)` and
/// `Ptr.from_int(n)` (6.4): what a SQLite binding needs.
const BYTES: &str = "extern c \"c\" {
    fn memcmp(a: Bytes, b: Bytes, n: UInt64) -> Int32
    fn memchr(s: Ptr<UInt8>, c: Int32, n: UInt64) -> Ptr<UInt8>
}

fn main() {
    unsafe {
        print(memcmp(\"abc\".bytes(), \"abd\".bytes(), 3) < 0)
        const p: Ptr<UInt8> = alloc(4)
        p[0] = 104
        p[1] = 105
        p[2] = 0
        p[3] = 7
        print(p.bytes(4).to_list())
        const found = memchr(p, 105, 4)
        print(found.bytes(2).to_list())
        const nothing: Ptr<UInt8> = Ptr.from_int(0)
        print(nothing.text())
        free(p)
    }
}
";

#[test]
fn bytes_go_to_c_and_come_back() {
    let expected = "true
[104, 105, 0, 7]
[105, 0]
none
";
    let dir = native("bytes", BYTES, expected);
    std::fs::remove_dir_all(&dir).unwrap();
}
