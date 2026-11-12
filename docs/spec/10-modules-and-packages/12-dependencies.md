# 10.12 Dependencies

A project uses another Kavach package by naming it in `[dependencies]`. The package is a library (`src/lib.kav`, 10.3), found in a folder or in a git repository. There is no package registry yet: a version number such as `stripe = "1.2"` is an error that says so.

```toml
[dependencies]
geometry = { path = "../geometry" }                                   # a folder
money = { git = "https://github.com/example/money", tag = "v1.0.0" }  # a git repository
```

```kavach
use geometry                   // the package's src/lib.kav
use geometry.shapes            // its src/shapes.kav

fn main() {
    print(geometry.area(2.0, 3.5))
}
```

- **Names.** The name in `[dependencies]` is the package's module name: lowercase letters, digits and `_`. Its `src/lib.kav` is the module `geometry`, and `src/shapes.kav` is `geometry.shapes`. Only `pub` items can be used from outside the package (10.1).
- **Inside a package,** `use shapes` names the package's own module `geometry.shapes` first, so a package's code means the same whichever project uses it.
- **Git.** `tag`, `branch` or `rev` (a commit) says which code to use; without one, the repository's default branch. Fetched repositories are kept in `kavach_modules/<name>/` in the project's folder, one folder for each package, as npm keeps `node_modules/`. Never commit it (`kavach new` puts it in `.gitignore`): `kavach.lock` is enough to fetch it again. A path dependency is read where it is, not copied.
- **A folder of a repository.** `dir` names the folder of the repository that holds the package, when one repository holds several: `router = { git = "https://github.com/example/rath", tag = "v1.0.0", dir = "packages/router" }`. It is a path inside the repository, written with `/`: one that starts with `/` or `-`, or has a `..` part, is refused.
- **`kavach.lock`** records the exact commit of each git dependency (and the folder of each path one). It is written when a dependency is first fetched, and commit it: every build then uses the same code, and the network is used only when something is missing. `kavach update` fetches each git dependency again and rewrites the lock.
- **Code from outside is handled with care.** Fetching a dependency only copies its files: none of its code runs then, as Kavach packages have no install scripts. A `git` source must be `https://`, `ssh://`, `user@host:path` or a folder; `http://`, `git://` and other transports are refused, since they are unencrypted or run commands. A URL holding a password or token is refused too, as `kavach.toml` and `kavach.lock` are committed: git finds the credentials for a private repository itself, from a credential helper or an SSH key. A fetched package can't name a `path` outside its own folder. A name that Windows keeps for a device (`con`, `nul`, `com1`…) is refused, so every project builds on every system.

  ```toml
  money = { git = "https://github.com/example/money" }              # fetched
  money = { git = "http://github.com/example/money" }               # error: unencrypted
  money = { git = "https://me:token@github.com/example/money" }     # error: the token would be committed
  ```

- **Dependencies of dependencies** are used too. Two different packages with one name are an error: rename one in its `[dependencies]`. A module in `src/` with a dependency's name is an error too (10.2).
- **`unsafe`.** A package that sets `unsafe = true` (6.4) is used only when the project allows it in its entry: `geometry = { path = "../geometry", unsafe = true }`. Otherwise building stops and says so.
- **Warnings** in a dependency's code are not shown: they are its authors' to fix. Errors are.
- A dependency's tests (`*.test.kav`) and `src/main.kav` are not compiled into the project that uses it, and `kavach test` runs none of its examples or generated tests: they are its own package's to run.
- **Capabilities are granted per dependency.** A dependency may use only the capabilities (8.1) its entry lists in `can`; an entry without `can` grants none, so the package must be pure. Using one more is a compile error that names the function and the capability. A dependency's own dependencies get at most what it was granted. `kavach add` lists the capabilities the package uses and writes them into `can`, so the person adding it sees them ([decision 0084](../../decisions/0084-environment-variables-and-granted-capabilities.md)).

  ```toml
  [dependencies]
  money = { git = "https://github.com/example/money", tag = "v1.0.0", can = ["Net"] }
  geometry = { path = "../geometry" }          # pure: no capabilities
  ```
