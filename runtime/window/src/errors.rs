// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Windowing errors as a program sees them (spec 14.22).

/// A windowing error as a program shows it: winit writes where in its own
/// source it failed (`os error at /home/…/winit-0.30.13/src/…:765: …`),
/// which names the build machine's folders and means nothing to a user.
pub(crate) fn shown_error(e: &dyn std::fmt::Display) -> String {
    let text = e.to_string();
    match text.strip_prefix("os error at ") {
        Some(rest) => match rest
            .split_once(".rs:")
            .and_then(|(_, r)| r.split_once(": "))
        {
            Some((_, why)) => why.to_string(),
            None => text,
        },
        None => text,
    }
}

#[cfg(test)]
mod plain_tests {
    #[test]
    fn a_windowing_error_names_no_build_folder() {
        // Whatever folder winit was built in, on any system, goes.
        for raw in [
            "os error at /home/someone/.cargo/registry/src/x/winit-0.30.13/src/platform_impl/linux/mod.rs:765: neither WAYLAND_DISPLAY nor DISPLAY is set.",
            r"os error at C:\Users\someone\.cargo\registry\src\x\winit-0.30.13\src\platform_impl\windows\mod.rs:765: neither WAYLAND_DISPLAY nor DISPLAY is set.",
        ] {
            assert_eq!(
                super::shown_error(&raw),
                "neither WAYLAND_DISPLAY nor DISPLAY is set."
            );
        }
        assert_eq!(super::shown_error(&"no screen"), "no screen");
    }
}
