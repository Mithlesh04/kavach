# Setting up a Windows machine

How to set up a Windows 10 or 11 machine to run Kavach's tests and benchmarks.

Tests run on Windows itself. Benchmarks run in WSL, the Linux system built into Windows, because they compare Kavach with `g++ -O2` on Linux.

## Part 1: Testing

### 1. Install the tools

1. **Git for Windows** from [git-scm.com](https://git-scm.com). It also installs Git Bash, which the repo scripts use.
2. **Visual Studio Build Tools 2022** from [visualstudio.microsoft.com/downloads](https://visualstudio.microsoft.com/downloads/). In the installer, tick **Desktop development with C++**. Rust needs it, and `kavach build` links programs with its `link.exe`.
3. **Rust:** run `rustup-init.exe` from [rustup.rs](https://rustup.rs) and accept the defaults.

### 2. Get the code and run the tests

Open a **new** terminal, so it sees the tools you just installed:

```
rustup component add llvm-tools
git clone https://github.com/Mithlesh04/kavach.git
cd kavach
cargo test -p kavach-cli
```

`llvm-tools` gives `kavach build` its LLVM backend ([decision 0018](../decisions/0018-llvm-backend-through-llvm-tools.md)). Without it, Kavach falls back to the slower Cranelift backend and says nothing.

`cargo test -p kavach-cli` runs the conformance tests. `cargo test` runs every test in the repository.

### 3. Check it works

```
cargo run -p kavach-cli -- run examples/01-hello/hello.kav
```

It should print a greeting.

If you will commit, install the commit hook once: `powershell -File scripts/install-hooks.ps1`.

## Part 2: Benchmarking

### 1. Install WSL

In PowerShell **as administrator**:

```
wsl --install -d Ubuntu-24.04
```

Restart Windows. Then open **Ubuntu** from the Start menu and choose a user name and password.

### 2. Install the tools in Ubuntu

In the Ubuntu window:

```
sudo apt update
sudo apt install -y build-essential python3 nodejs git
curl https://sh.rustup.rs -sSf | sh
```

Accept the defaults, then close and reopen Ubuntu. Then:

```
rustup component add llvm-tools
rustup component list --installed | grep llvm-tools
git clone https://github.com/Mithlesh04/kavach.git ~/kavach
```

The second line must print `llvm-tools-...`. Python and Node are optional: the benchmarks in `chains/` and `json/` also time them.

Keep this copy in `~/kavach`, separate from the Windows copy. Building from `/mnt/c` or `/mnt/d` is slow, and the two systems would share one `target` folder.

### 3. Run the benchmarks

In the Ubuntu window:

```
cd ~/kavach
sh scripts/run-benchmarks.sh
sh scripts/run-benchmarks.sh target/release/kavach benchmarks/math
```

The first command times [benchmarks/loops](../../benchmarks/loops/README.md) and builds `kavach` first if needed. The second times another folder: `math`, `everyday`, `chains`, `tasks` or `json` ([the list](../../benchmarks/README.md#benchmarks)).

### 4. Share the results

- Send the machine with the numbers: CPU, Windows version, `g++ --version` and the date.
- Compare the **ratio** column (Kavach's time divided by C++'s), not the milliseconds: times differ between CPUs.
- Plug in the laptop and close other programs while it runs. Timings vary by about 10% between runs.

## Updating later

In both copies (Windows and Ubuntu):

```
git pull
cargo build --release -p kavach-cli
```
