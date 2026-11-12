# 10.13 Environment variables and .env files

`env(name)` reads an environment variable. It needs `Env` (8.1) and fails when the variable is not set. Its value is a **secret** (8.7): it can't be printed, logged or put in a prompt, since keys and passwords usually come this way. A value that isn't secret, such as a port, is revealed with `reveal(…)`, which is visible in review.

```kavach
fn main() ! Error uses Env, Console {
    const key = try env("STRIPE_KEY")          // a secret String
    print(key)                                 // compile error: a secret
    const port = try Int(reveal(try env("PORT")))  // allowed
    print("listening on {port}")
}
```

A **`.env` file** gives a program variables without setting them in the shell. It is never committed: `kavach new` puts it in `.gitignore`.

```
# .env
STRIPE_KEY=sk_test_123
PORT=8080
GREETING="Hello, world"
```

- **Lines** are `NAME=value`, and `export NAME=value` is accepted too. A name is letters, digits and `_`, not starting with a digit. `#` starts a comment line, and blank lines are skipped. A value in double quotes may hold `\n`, `\"` and `\\`; a value in single quotes is taken as written. Nothing is expanded: `$HOME` stays as written. Any other line is an error that names the file and line.
- **Which file:** `--env-file <path>`, else `[env] file` in `kavach.toml`, else `.env` beside `kavach.toml`, when there is one. A dependency's `.env` is never read.
- **Single variables** come from `--env NAME=value` (repeatable) and from `[env.vars]` in `kavach.toml`. They override the file.
- **Values are text,** as environment variables are on every system. In a `.env` file and in `--env`, every value is text: `PORT=8080` gives `"8080"`. In `[env.vars]`, text, a number, `true` or `false`, and a date are accepted, each given as its text (`8080` gives `"8080"`, `1.5` gives `"1.5"`, `true` gives `"true"`). A list or a table is an error: write a list as text, such as `"a,b,c"`, and split it in the program. The program converts what it reads:

  ```kavach
  const port = try Int(reveal(try env("PORT")))              // "8080" → 8080
  const debug = try Bool(reveal(env("DEBUG") catch "false"))  // "true" → true
  const hosts = reveal(try env("HOSTS")).split(",")           // "a,b,c" → ["a", "b", "c"]
  ```
- **`kavach run` and `kavach test`** read the file when they start the program.
- **`kavach build`** copies the file, with `[env.vars]` and `--env` written into it, beside the executable: `.kavach/bin/.env`. The executable reads the `.env` beside it the first time it reads a variable, so a program that never calls `env` pays nothing for it; a file it can't read, or a wrong line in it, ends the program there with exit code 2. Its values are never compiled into the executable, which is shared and copied. `--no-env-file` builds without one.
- **A built program** reads the file named by the variable `KAVACH_ENV_FILE` instead, when it is set, so a container can keep the file elsewhere: `docker run -e KAVACH_ENV_FILE=/run/secrets/app.env app`.
- **Which value wins,** strongest first: `--env` given to `kavach run` or `kavach test`; then the real environment (the shell, `docker run -e`, a service manager); then `[env.vars]`; then the file. So the place a built program runs always has the last word.

```toml
[env]
file = ".env.local"                # instead of .env
vars = { LOG_LEVEL = "info" }      # overrides the file; not for secrets, as kavach.toml is committed
```

```
kavach run --env-file .env.test --env PORT=9000
kavach build --env LOG_LEVEL=warn
docker run -e PORT=80 my-app       # beats the copied .env
```
