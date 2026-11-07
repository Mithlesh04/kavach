//! A `gpu fn` that calls a plain `fn` says whether that function fits the
//! GPU's rules, or what keeps it off (spec 15.1).

fn first_error(src: &str) -> String {
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    c.messages
        .iter()
        .find(|m| m.severity == kavach_driver::Severity::Error)
        .map(|m| m.text.clone())
        .unwrap_or_default()
}

#[test]
fn a_plain_fn_called_from_a_gpu_fn_says_whether_it_fits() {
    let call = |helper: &str| {
        format!(
            "{helper}\n\ngpu fn run(x: Float32) -> Float32 {{\n    helper(x)\n}}\n\nfn main() {{}}\n"
        )
    };
    let fits = first_error(&call("fn helper(x: Float32) -> Float32 {\n    x * 2.0\n}"));
    assert!(
        fits.contains("it fits the GPU's rules. Mark it `gpu fn helper`"),
        "{fits}"
    );
    let prints = first_error(&call(
        "fn helper(x: Float32) -> Float32 {\n    print(x)\n    x\n}",
    ));
    assert!(
        prints.contains("can't be one, since it uses `print`"),
        "{prints}"
    );
}

#[test]
fn a_gpu_fn_cant_fail() {
    // One message saying why, not the usual "add `! Error`".
    let src = "fn g(x: Int32) -> Int32 ! Error {\n    x\n}\n\ngpu fn f(x: Int32) -> Int32 {\n    try g(x)\n}\n\nfn main() {}\n";
    let c = kavach_driver::compile(src, "main.kav", Default::default());
    let errors: Vec<&str> = c
        .messages
        .iter()
        .filter(|m| m.severity == kavach_driver::Severity::Error)
        .map(|m| m.text.as_str())
        .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("can't use `try`, `catch` or `fail`: it can't fail"),
        "{errors:?}"
    );
}
