// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The system's dialogs for `std.app` (spec 14.30, decision 0118),
//! through `rfd`: open, save, folder, message and confirm. Each waits for
//! the person. On a machine with no screen, `rfd` gives what cancelling
//! gives: no path, and no for a question.
//!
//! These need a person at the screen, so the tests can't show them; they
//! check what each gives back is passed on as `std.app` reads it.

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

/// Which dialog `std.app` asks for, by number.
pub const OPEN: i64 = 0;
pub const SAVE: i64 = 1;
pub const FOLDER: i64 = 2;
pub const MESSAGE: i64 = 3;
pub const CONFIRM: i64 = 4;

/// Shows dialog `which` with `title` and `text` (a file name to start
/// from for `SAVE`, the words for `MESSAGE` and `CONFIRM`): the path
/// chosen, `"yes"` for a confirmed question, or `""` for none.
pub fn show(which: i64, title: &str, text: &str) -> String {
    let path = |p: Option<std::path::PathBuf>| {
        p.map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    match which {
        OPEN => path(FileDialog::new().set_title(title).pick_file()),
        SAVE => path(
            FileDialog::new()
                .set_title(title)
                .set_file_name(text)
                .save_file(),
        ),
        FOLDER => path(FileDialog::new().set_title(title).pick_folder()),
        MESSAGE => {
            MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title(title)
                .set_description(text)
                .set_buttons(MessageButtons::Ok)
                .show();
            String::new()
        }
        _ => {
            let answer = MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title(title)
                .set_description(text)
                .set_buttons(MessageButtons::YesNo)
                .show();
            yes(&answer).to_string()
        }
    }
}

/// `"yes"` for a Yes, `""` for anything else.
fn yes(answer: &MessageDialogResult) -> &'static str {
    match answer {
        MessageDialogResult::Yes => "yes",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::yes;
    use rfd::MessageDialogResult;

    /// Only Yes is a yes: a closed dialog, No and Cancel are no.
    #[test]
    fn only_yes_confirms() {
        assert_eq!(yes(&MessageDialogResult::Yes), "yes");
        for other in [
            MessageDialogResult::No,
            MessageDialogResult::Cancel,
            MessageDialogResult::Ok,
        ] {
            assert_eq!(yes(&other), "");
        }
    }
}
