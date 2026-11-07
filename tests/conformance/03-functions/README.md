# 3. Functions

Conformance tests for spec chapter [3. Functions](../../../docs/spec/03-functions.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 3.1 | [last-expression-is-returned.kav](last-expression-is-returned.kav) | A function returns its last expression. | runs |
| 3.1 | [no-arrow-call-has-no-value.kav](no-arrow-call-has-no-value.kav) | A call to a function without `->` can't be used as a value. | compile error |
| 3.1 | [no-arrow-must-not-end-with-a-value.kav](no-arrow-must-not-end-with-a-value.kav) | A function without `->` returns nothing, so its body must not end with a value. | compile error |
| 3.1 | [parameters-cannot-be-reassigned.kav](parameters-cannot-be-reassigned.kav) | Parameters can't be reassigned. | compile error |
| 3.1 | [return-exits-early.kav](return-exits-early.kav) | `return` exits a function early. | runs |
| 3.1 | [no-rest-parameters.kav](no-rest-parameters.kav) | Separate values for a function's one list parameter are a compile error: put them in a list. | compile error |
| 3.2 | [named-arguments-and-defaults.kav](named-arguments-and-defaults.kav) | Any parameter can be passed by name; parameters may have defaults. | runs |
| 3.2 | [positional-arguments-come-first.kav](positional-arguments-come-first.kav) | Positional arguments must come before named ones. | compile error |
| 3.3 | [dot-shorthand.kav](dot-shorthand.kav) | In an argument, an expression starting with `.` is a lambda over one value. | runs |
| 3.3 | [lambda-types-come-from-use.kav](lambda-types-come-from-use.kav) | Lambda parameter types are inferred from where the lambda is used. | compile error |
| 3.3 | [lambdas-copy-what-they-use.kav](lambdas-copy-what-they-use.kav) | A lambda keeps a copy of the outside values it uses, taken when it is created. | runs |
| 3.3 | [lambdas-change-a-copy.kav](lambdas-change-a-copy.kav) | A lambda changing an outside variable, with `+=` or `push`, changes a copy made for each call, with a warning. | warning |
| 3.3 | [lambdas.kav](lambdas.kav) | Lambdas are written `x => …`, `(a, b) => …` or with a block body. | runs |
| 3.3 | [pipelines-run-step-by-step.kav](pipelines-run-step-by-step.kav) | `map` runs its lambda on every item before `filter` starts, so lambdas that print show each step in full, in order. | runs |
| 3.3, 5.8 | [fused-pipelines-panic-in-order.kav](fused-pipelines-panic-in-order.kav) | `map`, `filter` and `sum` in a row panic where the first failing step is as written: here `map` overflows on the last item, though the sum would overflow sooner. | panic |
| 3.4 | [functions-are-values.kav](functions-are-values.kav) | Functions are values and can be stored in variables. | runs |
| 3.4 | [function-fields-are-called.kav](function-fields-are-called.kav) | A record field holding a function is called like a method: `h.on(x)`. | runs |
| 3.4, 2.9 | [generic-functions-are-values.kav](generic-functions-are-values.kav) | A generic function used as a value takes its types from where it is used. | runs |
| 3.5 | [signature-order-is-fixed.kav](signature-order-is-fixed.kav) | The parts of a signature always come in the same order: ->, !, uses, require, ensure, example. | compile error |
| 3.7 | [discard-with-underscore.kav](discard-with-underscore.kav) | `const _ =` throws a value away on purpose. | runs |
| 3.7 | [unused-result-is-an-error.kav](unused-result-is-an-error.kav) | A call's value must be used or discarded with `const _ =`. | compile error |
| 3.8, 5.6 | [stack-overflow-panics.kav](stack-overflow-panics.kav) | Running out of stack is a panic, at the line of the call that went too deep. | panic |
| 3.8 | [deep-recursion-runs.kav](deep-recursion-runs.kav) | Recursion runs as deep as the stack allows: 4,000 calls deep, in every build. | runs |
| 3.9 | [inout-changes-the-callers-variable.kav](inout-changes-the-callers-variable.kav) | An `inout` parameter's last value goes back to the caller's variable, field or item. | runs |
| 3.9 | [inout-needs-a-let.kav](inout-needs-a-let.kav) | An `inout` argument must be able to change: not a `const`. | compile error |
| 3.9 | [inout-takes-no-plain-value.kav](inout-takes-no-plain-value.kav) | A value or a call can't be an `inout` argument. | compile error |
| 3.9 | [inout-variables-are-not-given-twice.kav](inout-variables-are-not-given-twice.kav) | An `inout` variable can't be another argument of the same call. | compile error |
| 3.9, 3.4 | [inout-functions-are-not-values.kav](inout-functions-are-not-values.kav) | A function with `inout` parameters isn't a function value. | compile error |
| 3.9, 5.2 | [inout-keeps-the-old-value-on-failure.kav](inout-keeps-the-old-value-on-failure.kav) | When the function fails, the places keep their old values. | runs |
| 3.9, 2.15 | [inout-through-an-interface.kav](inout-through-an-interface.kav) | A call through an interface value changes the caller's variable. | runs |
| 3.9 | [inout-passes-on-to-other-calls.kav](inout-passes-on-to-other-calls.kav) | An `inout` parameter passed on as `inout`, to another function or a recursive call, changes the first caller's variable. | runs |
| 3.9, 3.3 | [inout-is-a-copy-in-a-lambda.kav](inout-is-a-copy-in-a-lambda.kav) | A lambda changing an `inout` parameter changes its own copy, with a warning. | warning |
