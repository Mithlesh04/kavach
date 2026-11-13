# 7. Contracts

Conformance tests for spec chapter [7. Contracts](../../../docs/spec/07-contracts.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 7.1, 7.4 | [require-failure-panics.kav](require-failure-panics.kav) | A false `require` is a panic that shows the contract. | panic |
| 7.2 | [ensure-sees-values-before-the-call.kav](ensure-sees-values-before-the-call.kav) | In `ensure`, parameters are the values from before the call. | runs |
| 7.2 | [ensure-skipped-on-error.kav](ensure-skipped-on-error.kav) | `ensure` is checked only when the function succeeds. | runs |
| 7.2, 7.4 | [ensure-failure-panics.kav](ensure-failure-panics.kav) | A false `ensure` is a panic that shows the contract. | panic |
| 7.3, 7.4 | [invariant-checked-on-change.kav](invariant-checked-on-change.kav) | An invariant is checked when a value is changed. | panic |
| 7.3, 7.4 | [invariant-checked-on-create.kav](invariant-checked-on-create.kav) | An invariant is checked when a value is created. | panic |
| 7.3 | [invariants-call-methods-on-fields.kav](invariants-call-methods-on-fields.kav) | An invariant may call methods on the fields it names, in native code as in the interpreter. | panic |
| 7.5 | [contracts-must-be-pure.kav](contracts-must-be-pure.kav) | A contract must not use any capability. | compile error |
| 7.6 | [examples-pass.kav](examples-pass.kav) | `kavach test` runs every example, including `fails` examples. | kavach test |
| 7.6 | [wrong-example-fails-tests.kav](wrong-example-fails-tests.kav) | A false example makes `kavach test` fail. | kavach test |
| 7.6 | [wrong-method-example-fails-tests.kav](wrong-method-example-fails-tests.kav) | A false example on a method makes `kavach test` fail. | kavach test |
| 7.7 | [a-result-that-always-breaks-an-ensure.kav](a-result-that-always-breaks-an-ensure.kav) | A known value a function returns that makes an `ensure` false is a compile error. | compile error |
| 7.7 | [a-result-the-compiler-cant-know-is-checked-when-it-runs.kav](a-result-the-compiler-cant-know-is-checked-when-it-runs.kav) | An `ensure` is checked at the run for what the compiler can't know; a known result that keeps it is no error. | panic |
| 7.7 | [a-call-that-always-breaks-a-require.kav](a-call-that-always-breaks-a-require.kav) | A call whose known arguments make a `require` false is a compile error: it would always panic. | compile error |
| 7.7 | [a-call-that-always-breaks-a-require-on-floats.kav](a-call-that-always-breaks-a-require-on-floats.kav) | A known float argument that makes a `require` false is a compile error too. | compile error |
