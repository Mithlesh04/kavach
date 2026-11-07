//! A library built with `kavach build --lib` may be called from several
//! threads at once (decisions 0026 and 0035): calls compute at the same
//! time, each on its own memory, and take the library lock at their first
//! print, so the answers stay right and printed lines stay whole.

use std::process::Command;

const LIBRARY: &str = "pub fn fib(n: Int) -> Int {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

pub fn double(n: Int) -> Int {
    n * 2
}

pub fn words(n: Int) -> Int {
    let items: [String] = []
    for i in 0..n {
        items.push(\"word {i}\")
    }
    let total = 0
    for w in items {
        total += w.len()
    }
    total
}

pub fn shout(n: Int) -> Int uses Console {
    const items = [n, n + 1, n + 2]
    print(\"call {n} says {items}\")
    n
}
";

const CALLER: &str = "import ctypes, sys, threading

lib = ctypes.CDLL(sys.argv[1])
lib.counts_fib.argtypes = [ctypes.c_int64]
lib.counts_fib.restype = ctypes.c_int64
lib.counts_double.argtypes = [ctypes.c_int64]
lib.counts_double.restype = ctypes.c_int64
lib.counts_words.argtypes = [ctypes.c_int64]
lib.counts_words.restype = ctypes.c_int64
lib.counts_shout.argtypes = [ctypes.c_int64]
lib.counts_shout.restype = ctypes.c_int64

wrong = []

def hammer(seed):
    for i in range(200):
        n = (seed + i) % 20
        if lib.counts_fib(n) != fibs[n]:
            wrong.append(('fib', n))
        if lib.counts_double(n) != 2 * n:
            wrong.append(('double', n))
        w = 100 * n
        if lib.counts_words(w) != sum(len(f'word {k}') for k in range(w)):
            wrong.append(('words', w))
        if i % 20 == 0 and lib.counts_shout(n) != n:
            wrong.append(('shout', n))

fibs = [0, 1]
for i in range(2, 20):
    fibs.append(fibs[i - 1] + fibs[i - 2])

threads = [threading.Thread(target=hammer, args=(s,)) for s in range(8)]
for t in threads:
    t.start()
for t in threads:
    t.join()
print('wrong:', len(wrong), flush=True)
";

#[test]
fn a_library_survives_calls_from_several_threads() {
    let dir = std::env::temp_dir().join(format!("kavach-threads-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"counts\"\n").unwrap();
    std::fs::write(dir.join("src/lib.kav"), LIBRARY).unwrap();
    let built = Command::new(env!("CARGO_BIN_EXE_kavach"))
        .args(["build", "--lib"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let file = if cfg!(windows) {
        "counts.dll"
    } else {
        "libcounts.so"
    };
    std::fs::write(dir.join("call.py"), CALLER).unwrap();
    let python = if cfg!(windows) { "python" } else { "python3" };
    let ran = Command::new(python)
        .arg("-I")
        .arg(dir.join("call.py"))
        .arg(dir.join(".kavach/lib").join(file))
        .output()
        .expect("Python, to call the library");
    let out = String::from_utf8_lossy(&ran.stdout).replace("\r\n", "\n");
    // Python's output and the library's are buffered apart, so Python's
    // line may come anywhere.
    let (lines, python): (Vec<&str>, Vec<&str>) =
        out.lines().partition(|l| !l.starts_with("wrong:"));
    assert_eq!(
        python,
        ["wrong: 0"],
        "{out}{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    // 8 threads print 10 times each, every line whole.
    assert_eq!(lines.len(), 80, "{out}");
    for line in lines {
        let n: i64 = line
            .strip_prefix("call ")
            .and_then(|l| l.split(' ').next())
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("a broken line: {line:?}"));
        assert_eq!(line, format!("call {n} says [{n}, {}, {}]", n + 1, n + 2));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
