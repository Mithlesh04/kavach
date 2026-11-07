//! `kavach build --lib` (spec 10.10, decision 0026): a library package
//! becomes a shared library with a C header, and another language calls
//! it. Python's `ctypes` is the caller here, as it would be for anyone.

use std::process::Command;

const LIBRARY: &str = "/// The area of a rectangle.
pub fn area(width: Float, height: Float) -> Float {
    width * height
}

/// The `n`th Fibonacci number, through a list, printing as it goes.
pub fn fib(n: Int) -> Int uses Console {
    let xs = [0, 1]
    for i in 2..=n {
        xs.push(xs[i - 1] + xs[i - 2])
    }
    print(\"fib({n}) from Kavach\")
    xs[n]
}

pub fn is_even(n: Int32) -> Bool {
    n % 2 == 0
}

pub fn greet(name: String) -> String {
    \"hello {name}, {name.len()} letters\"
}

pub type AgeError = Negative | TooOld(age: Int)

pub fn check_age(age: Int) -> Int ! AgeError {
    if age < 0 {
        fail AgeError.Negative
    }
    if age > 150 {
        fail AgeError.TooOld(age: age)
    }
    age
}

pub fn label(age: Int) -> String ! AgeError {
    const a = try check_age(age)
    \"age {a}\"
}

pub fn total(xs: [Int]) -> Int {
    xs.sum(x => x)
}

fn helper() -> Int {
    1
}
";

const CALLER: &str = "import ctypes, sys
sys.stdout.reconfigure(encoding='utf-8')
lib = ctypes.CDLL(sys.argv[1])
lib.geometry_area.argtypes = [ctypes.c_double, ctypes.c_double]
lib.geometry_area.restype = ctypes.c_double
lib.geometry_fib.argtypes = [ctypes.c_int64]
lib.geometry_fib.restype = ctypes.c_int64
lib.geometry_is_even.argtypes = [ctypes.c_int32]
lib.geometry_is_even.restype = ctypes.c_bool
print(lib.geometry_area(2.0, 3.5), flush=True)
print(lib.geometry_fib(50), flush=True)
print(lib.geometry_is_even(7), lib.geometry_is_even(-4), flush=True)
lib.geometry_greet.argtypes = [ctypes.c_char_p]
lib.geometry_greet.restype = ctypes.c_void_p
lib.geometry_free.argtypes = [ctypes.c_void_p]
text = lib.geometry_greet('Zoë'.encode('utf-8'))
print(ctypes.string_at(text).decode('utf-8'), flush=True)
lib.geometry_free(text)
lib.geometry_check_age.argtypes = [ctypes.c_int64, ctypes.POINTER(ctypes.c_int64)]
lib.geometry_check_age.restype = ctypes.c_int32
lib.geometry_last_error.restype = ctypes.c_void_p
age = ctypes.c_int64(0)
print(lib.geometry_check_age(30, ctypes.byref(age)), age.value, flush=True)
code = lib.geometry_check_age(200, ctypes.byref(age))
why = lib.geometry_last_error()
print(code, ctypes.string_at(why).decode('utf-8'), flush=True)
lib.geometry_free(why)
print(lib.geometry_check_age(-1, None), flush=True)
lib.geometry_label.argtypes = [ctypes.c_int64, ctypes.POINTER(ctypes.c_void_p)]
lib.geometry_label.restype = ctypes.c_int32
out = ctypes.c_void_p()
print(lib.geometry_label(40, ctypes.byref(out)), ctypes.string_at(out.value).decode('utf-8'), flush=True)
lib.geometry_free(out)
print(lib.geometry_label(40, None), lib.geometry_label(-5, None), flush=True)
";

#[test]
fn other_languages_call_a_kavach_library() {
    let dir = std::env::temp_dir().join(format!("kavach-lib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"geometry\"\n").unwrap();
    std::fs::write(dir.join("src/lib.kav"), LIBRARY).unwrap();
    // Inside the project, `--lib` builds `src/lib.kav` into `.kavach/lib/`.
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["build", "--lib"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&built.stderr);
    assert!(built.status.success(), "{said}");
    assert!(
        said.contains("not exported: `total`: it takes `[Int]`"),
        "{said}"
    );
    let out = dir.join(".kavach/lib");
    let header = std::fs::read_to_string(out.join("geometry.h")).unwrap();
    for line in [
        "double geometry_area(double width, double height);",
        "int64_t geometry_fib(int64_t n);",
        "bool geometry_is_even(int32_t n);",
        "char* geometry_greet(const char* name);",
        "void geometry_free(char* text);",
        "#define GEOMETRY_AGE_ERROR_NEGATIVE 1",
        "#define GEOMETRY_AGE_ERROR_TOO_OLD 2",
        "int32_t geometry_check_age(int64_t age, int64_t* result);",
        "int32_t geometry_label(int64_t age, char** result);",
        "char* geometry_last_error(void);",
    ] {
        assert!(header.contains(line), "{header}");
    }
    assert!(
        !header.contains("helper") && !header.contains("total"),
        "{header}"
    );
    let file = if cfg!(windows) {
        "geometry.dll"
    } else {
        "libgeometry.so"
    };
    std::fs::write(dir.join("call.py"), CALLER).unwrap();
    let python = if cfg!(windows) { "python" } else { "python3" };
    let ran = Command::new(python)
        .arg("-I")
        .arg(dir.join("call.py"))
        .arg(out.join(file))
        .output()
        .expect("Python, to call the library");
    let said = String::from_utf8_lossy(&ran.stderr);
    // Python ends lines with `\r\n` on Windows.
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout).replace("\r\n", "\n"),
        "7.0\nfib(50) from Kavach\n12586269025\nFalse True\nhello Zoë, 3 letters\n\
         0 30\n2 TooOld(age: 200)\n1\n0 age 40\n0 1\n",
        "{said}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
