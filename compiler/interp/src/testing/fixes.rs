//! Crash fix suggestions (spec 11.10). A panic in a generated case is a
//! bug; these are the ways to fix it, each with when to choose it. They
//! are only shown: the developer picks one, because only they know what
//! the input means.

/// Suggested fixes for a panic with `message`, raised at code `at`, in a
/// function whose result type is `ret`.
pub(crate) fn suggest(message: &str, at: &str, ret: &str) -> Vec<String> {
    let error = format!(
        "Return an error. Change the signature to `-> {ret} ! SomeError` and `fail` when the value is bad.\n       Choose this if the value can come from users, files, the network or AI."
    );
    let require = |condition: &str| {
        format!(
            "Forbid it. Add: require {condition}\n       Choose this if such a value here is always a bug in the calling code."
        )
    };
    if message.starts_with("division by zero") {
        let divisor = at.rsplit(['/', '%']).next().unwrap_or("divisor").trim();
        return vec![
            error,
            require(&format!("{divisor} != 0")),
            format!(
                "Use a default. Add: if {divisor} == 0 {{ return … }}\n       Choose this if a default is a correct answer."
            ),
        ];
    }
    if message.starts_with("index") {
        return vec![
            "Use `list.get(i)`, which returns an optional, with `??` or `if let`.\n       Choose this if a missing item has a sensible default.".into(),
            require("i < list.len()"),
            error,
        ];
    }
    if message.contains("`.unwrap()` on `none`") {
        return vec![
            "Handle `none` with `if let`, `match` or `?? default` instead of `.unwrap()`.".into(),
            error,
        ];
    }
    if message.starts_with("integer overflow") || message == "decimal overflow" {
        return vec![
            require("…  (a condition that bounds how large the inputs can be)"),
            "Use a wider type, such as `Int` instead of `Int32`.".into(),
            "Use `checked_add` (and similar), which returns `none` on overflow.".into(),
        ];
    }
    if message.starts_with("contract failed: require") {
        let condition = message.trim_start_matches("contract failed: require ");
        return vec![
            format!("Copy that contract onto this function: require {condition}"),
            "Check the value before the call, and handle the bad case.".into(),
        ];
    }
    if message.starts_with("contract failed: ensure")
        || message.starts_with("contract failed: invariant")
    {
        return vec![
            "Fix the function body for this input, or make the contract say what is really meant."
                .into(),
            require("…  (a condition that rules this input out)"),
        ];
    }
    if message.contains("time budget") {
        return vec![
            "Find the loop or recursion that doesn't end for this input.".into(),
            require("…  (a condition that bounds the input)"),
        ];
    }
    if message.starts_with("stack overflow") {
        return vec![
            "Make sure the recursion reaches its base case for this input.".into(),
            "Use a loop instead of recursion.".into(),
        ];
    }
    vec![
        "If this can happen in production, turn the `panic` into an error variant and `fail` with it.".into(),
        require("…  (a condition that rules this input out)"),
    ]
}
