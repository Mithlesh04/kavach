# Approvals tests

Approvals that wait (spec 8.3, decision 0072), run as one test target: `cargo test -p kavach-cli --test approvals`.

| File | What it tests |
|---|---|
| `main.rs` | A person approves or denies a waiting call, and a request runs out of time |
| `setup.rs` | What every test here needs: the program, its project, starting it, answering and reading it |
| `restarts.rs` | Across a restart: an answer given while the program was down, events cut short, joined and equal events run again |
| `in_code.rs` | Another program answers the requests in code (spec 14.16) |
