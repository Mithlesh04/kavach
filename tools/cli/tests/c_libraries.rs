//! Libraries loaded while running (`extern c library`, spec 10.7): a
//! Kavach library built with `kavach build --lib` is loaded by another
//! program, in memory and built with each backend; a function it lacks
//! is a `LibraryError`; the interpreter says to run natively.

use std::process::Command;

const LIBRARY: &str = "pub(out) fn add(a: Int32, b: Int32) -> Int32 {
    a + b
}

pub(out) fn scale(x: Float, by: Float) -> Float {
    x * by
}

pub(out) fn greet(name: String) -> String {
    \"hello {name}\"
}
";

const PROGRAM: &str = "extern c library Adder {
    fn adder_add(a: Int32, b: Int32) -> Int32
    fn adder_scale(x: Float, by: Float) -> Float
    fn adder_greet(name: String) -> Ptr<UInt8>
    fn adder_free(text: Ptr<UInt8>)
}

extern c library Missing {
    fn adder_add(a: Int32, b: Int32) -> Int32
    fn adder_nothing() -> Int32
}

fn main() ! LibraryError {
    const path = \"LIBRARY\"
    const lib = unsafe { try Adder.open(path) }
    print(unsafe { lib.adder_add(2, 3) })
    print(unsafe { lib.adder_scale(1.5, 4.0) })
    unsafe {
        const text = lib.adder_greet(\"Zoë\")
        print(text.text() ?? \"none\")
        lib.adder_free(text)
    }
    unsafe {
        const no = Missing.open(path) catch e => {
            print(\"{e}\".replace(path, \"adder\"))
            return
        }
        print(no.adder_add(1, 1))
    }
}
";

const EXPECTED: &str = "5\n6.0\nhello Zoë\nthe library adder has no function `adder_nothing`\n";

fn kavach() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kavach"))
}

#[test]
fn a_program_loads_a_library_while_running() {
    let dir = std::env::temp_dir().join(format!("kavach-clib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("adder/src")).unwrap();
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(
        dir.join("adder/kavach.toml"),
        "[package]\nname = \"adder\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("adder/src/lib.kav"), LIBRARY).unwrap();
    let app = dir.join("app");
    std::fs::write(
        app.join("kavach.toml"),
        "[package]\nname = \"app\"\nunsafe = true\n",
    )
    .unwrap();
    let built = kavach()
        .args(["build", "--lib"])
        .current_dir(dir.join("adder"))
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let file = if cfg!(windows) {
        "adder.dll"
    } else {
        "libadder.so"
    };
    let library = dir.join("adder/.kavach/lib").join(file);
    // The path as a Kavach text: each `\` written twice.
    let quoted = library.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(app.join("main.kav"), PROGRAM.replace("LIBRARY", &quoted)).unwrap();
    let mut runs = Vec::new();
    let jit = kavach()
        .args(["run", "--native", "main.kav"])
        .current_dir(&app)
        .output();
    runs.push(("jit", jit.unwrap()));
    let exe = app.join(format!("app{}", std::env::consts::EXE_SUFFIX));
    for backend in ["llvm", "cranelift"] {
        let made = kavach()
            .env("KAVACH_BACKEND", backend)
            .args(["build", "main.kav", "-o"])
            .arg(&exe)
            .current_dir(&app)
            .output()
            .unwrap();
        assert!(
            made.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&made.stderr)
        );
        runs.push((backend, Command::new(&exe).output().unwrap()));
    }
    let interpreted = kavach()
        .args(["run", "main.kav"])
        .current_dir(&app)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    for (how, out) in runs {
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            EXPECTED,
            "{how}: {said}"
        );
    }
    let said = String::from_utf8_lossy(&interpreted.stderr);
    assert!(said.contains("a C library loaded while running"), "{said}");
}

/// Records by value in every way System V passes them, to a C library
/// compiled here: on the stack (more than 16 bytes), in a whole-number
/// and a float register, two floats in one, and through a hidden pointer.
const SHAPES_C: &str = "typedef struct { long long a, b, c; } Big;
typedef struct { long long n; double x; } Mixed;
typedef struct { float x, y; int k; } Small;
long long big_sum(Big b) { return b.a + b.b + b.c; }
Mixed mixed_twice(Mixed m) { m.n *= 2; m.x *= 2; return m; }
Small small_next(Small s) { s.x += 1; s.y += 2; s.k += 3; return s; }
Big big_make(long long v) { Big b = { v, v + 1, v + 2 }; return b; }
";

const SHAPES: &str = "type Big {
    a: Int
    b: Int
    c: Int
}

type Mixed {
    n: Int
    x: Float
}

type Small {
    x: Float32
    y: Float32
    k: Int32
}

extern c library Shapes {
    fn big_sum(b: Big) -> Int
    fn mixed_twice(m: Mixed) -> Mixed
    fn small_next(s: Small) -> Small
    fn big_make(v: Int) -> Big
}

fn main() ! LibraryError {
    unsafe {
        const lib = try Shapes.open(\"LIBRARY\")
        print(lib.big_sum(Big(a: 1, b: 20, c: 300)))
        const m = lib.mixed_twice(Mixed(n: 21, x: 1.25))
        print(\"{m.n} {m.x}\")
        const s = lib.small_next(Small(x: 0.5, y: 1.5, k: 7))
        print(\"{s.x} {s.y} {s.k}\")
        const b = lib.big_make(40)
        print(\"{b.a} {b.b} {b.c}\")
    }
}
";

#[test]
fn records_go_to_a_c_library_by_value() {
    // Needs a C compiler; Linux machines that build Kavach have `cc`.
    if cfg!(windows) || Command::new("cc").arg("--version").output().is_err() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("kavach-shapes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shapes.c"), SHAPES_C).unwrap();
    let library = dir.join("libshapes.so");
    let cc = Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&library)
        .arg(dir.join("shapes.c"))
        .output()
        .unwrap();
    assert!(
        cc.status.success(),
        "{}",
        String::from_utf8_lossy(&cc.stderr)
    );
    let program = SHAPES.replace("LIBRARY", &library.to_string_lossy());
    std::fs::write(dir.join("main.kav"), program).unwrap();
    std::fs::write(
        dir.join("kavach.toml"),
        "[package]\nname = \"shapes\"\nunsafe = true\n",
    )
    .unwrap();
    let expected = "321\n42 2.5\n1.5 3.5 10\n40 41 42\n";
    let mut runs = vec![(
        "jit",
        kavach()
            .args(["run", "--native", "main.kav"])
            .current_dir(&dir)
            .output()
            .unwrap(),
    )];
    for backend in ["llvm", "cranelift"] {
        let made = kavach()
            .env("KAVACH_BACKEND", backend)
            .args(["build", "main.kav", "-o", "shapes"])
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(
            made.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&made.stderr)
        );
        runs.push((backend, Command::new(dir.join("shapes")).output().unwrap()));
    }
    let _ = std::fs::remove_dir_all(&dir);
    for (how, out) in runs {
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            expected,
            "{how}: {said}"
        );
    }
}
