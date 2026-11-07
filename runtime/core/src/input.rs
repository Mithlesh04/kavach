//! Reading what a person types, unseen (`io.read_hidden`, spec 14.6,
//! decision 0036), for the interpreter and native code alike.

use std::io::IsTerminal;

/// A line typed without being shown, when standard input is a terminal;
/// an ordinary line otherwise, so a pipe or a test can answer it. `None`
/// at the end of input.
pub fn read_hidden() -> Option<String> {
    if std::io::stdin().is_terminal() {
        return rpassword::read_password().ok();
    }
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim_end_matches(['\n', '\r']).to_string()),
    }
}
