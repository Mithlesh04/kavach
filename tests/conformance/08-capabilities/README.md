# 8. Capabilities and agents

Conformance tests for spec chapter [8. Capabilities and agents](../../../docs/spec/08-capabilities-and-agents/README.md). Format: [../README.md](../README.md). Built-in capabilities (8.1) are checked at compile time; declared capabilities and agents (8.2–8.8) run in the interpreter, with native code to follow.

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 8.1 | [pub-functions-list-their-uses.kav](pub-functions-list-their-uses.kav) | A `pub` function must write its `uses` clause. | compile error |
| 8.1 | [uses-clause-limits-capabilities.kav](uses-clause-limits-capabilities.kav) | A function with a `uses` clause can only use the capabilities it lists. | compile error |
| 8.1 | [calls-need-the-callees-capabilities.kav](calls-need-the-callees-capabilities.kav) | A function may only call functions whose capabilities it also has. | compile error |
| 8.1 | [private-functions-infer-uses.kav](private-functions-infer-uses.kav) | A private function may leave out `uses`; the compiler infers it. | runs |
| 8.1 | [inferred-uses-reach-pub-functions.kav](inferred-uses-reach-pub-functions.kav) | Inferred capabilities count when a `pub` function calls a private one. | compile error |
| 8.1 | [debug-needs-no-capability.kav](debug-needs-no-capability.kav) | `debug()` needs no capability. | runs |
| 8.7, 2.11 | [reveal-lets-a-secret-through.kav](reveal-lets-a-secret-through.kav) | `reveal(value)` deliberately lets a secret through, where it would otherwise be refused. | runs |
| 8.1 | [reading-files-is-granted-apart.kav](reading-files-is-granted-apart.kav) | `uses Fs.read` allows reading files only. | compile error |
| 8.1 | [file-access-stays-inside-its-folder.kav](file-access-stays-inside-its-folder.kav) | With `uses Fs("folder")`, file operations (also in callees and tasks) must stay inside the folder, or fail with `Denied`. | runs |
| 8.1 | [rename-and-copy-stay-inside-the-folder.kav](rename-and-copy-stay-inside-the-folder.kav) | Inside `uses Fs("folder")`, both paths of `fs.rename` and `fs.copy` must be in the folder, or the call fails with `Denied` for the one outside. | runs |
| 8.1 | [methods-need-their-capabilities.kav](methods-need-their-capabilities.kav) | Calling a method needs the capabilities the method uses, as calling a function does. | compile error |
| 8.2 | [capability-errors-have-denied.kav](capability-errors-have-denied.kav) | Every capability method declares an error type with `Denied(reason: String)`. | compile error |
| 8.2 | [capability-calls-outside-agents-are-denied.kav](capability-calls-outside-agents-are-denied.kav) | Outside a running agent, a declared capability call fails with `Denied`. | runs |
| 8.3 | [agent-handlers-need-can.kav](agent-handlers-need-can.kav) | A handler may only use what its agent lists in `can`, also through functions it calls. | compile error |
| 8.8 | [agent-start-takes-each-implementation.kav](agent-start-takes-each-implementation.kav) | `start` takes one implementation for every declared capability the agent can use. | compile error |
| 8.8 | [agent-handlers-use-its-implementations.kav](agent-handlers-use-its-implementations.kav) | A handler's capability calls go to the implementations the agent was started with. | runs |
| 8.8, 5.6 | [a-handler-that-panics-fails-its-send.kav](a-handler-that-panics-fails-its-send.kav) | A handler that panics is stopped: the send fails with `panicked: … at …`, and the agent goes on serving. | runs |
| 8.8, 5.6 | [a-caught-panic-leaves-the-senders-values.kav](a-caught-panic-leaves-the-senders-values.kav) | After a send fails with a handler's panic, the sender goes on with all its own values as they were (on Windows, Cranelift's unwind tables once gave them back from the wrong slots). | runs |
| 8.3 | [limits-refuse-calls.kav](limits-refuse-calls.kav) | A call that breaks a `limit` rule is not executed; it fails with `Denied`, naming the rule. | runs |
| 8.3 | [limit-windows-count-calls-that-ran.kav](limit-windows-count-calls-that-ran.kav) | `sum(…)` and `count` count the calls that ran in the window, this one included. | runs |
| 8.3 | [approvals-need-a-person.kav](approvals-need-a-person.kav) | When an `approve` condition holds, a person must approve; with no one to ask the call is refused. | runs |
| 8.3 | [limit-rules-are-typed-like-code.kav](limit-rules-are-typed-like-code.kav) | `limit` rules and `approve` conditions are checked like code, with the limited method's parameter types. | runs |
| 8.7 | [secrets-stay-out-of-text.kav](secrets-stay-out-of-text.kav) | A secret can't be interpolated into text anywhere. | compile error |
| 8.7 | [secrets-go-only-to-secret-parameters.kav](secrets-go-only-to-secret-parameters.kav) | A secret can only be passed to a parameter or field marked `secret`. | compile error |
| 8.7 | [secret-parameters-and-copies-are-secret.kav](secret-parameters-and-copies-are-secret.kav) | A `secret` parameter is a secret, and so is a variable set from one. | compile error |
| 8.7 | [secrets-pass-to-secret-parameters.kav](secrets-pass-to-secret-parameters.kav) | A secret may go to a `secret` parameter; `reveal(…)` lets one through. | runs |
| 8.7 | [returning-a-secret-says-so.kav](returning-a-secret-says-so.kav) | A function that returns a secret says `-> secret T`; returning one without it is refused. | compile error |
| 8.7 | [secret-results-stay-secret.kav](secret-results-stay-secret.kav) | The result of a `-> secret T` function is a secret where it is called. | compile error |
| 8.7 | [secret-results-pass-where-secrets-may-go.kav](secret-results-pass-where-secrets-may-go.kav) | A secret result goes on to `secret` parameters, other `-> secret` functions and `reveal`. | runs |
| 8.7, 3.3 | [lambdas-return-no-secrets.kav](lambdas-return-no-secrets.kav) | A lambda can't return a secret. | compile error |
| 8.7, 14.10 | [tls-keys-take-secrets.kav](tls-keys-take-secrets.kav) | The private `key` of `net.listen_tls` and `net.connect_tls` takes a secret. | runs |
| 8.4 | [ask-fails-with-ai-error.kav](ask-fails-with-ai-error.kav) | `ask` fails with an `AiError`, so a function failing with `AiError` passes it on with `try`, and a `match` names its variants. | runs |
| 8.3, 8.4 | [agents-choose-their-model.kav](agents-choose-their-model.kav) | An agent's `model` and a call's `model:` are `Model` values known when the program is built: written there, or from a function with no parameters. | runs |
| 8.4 | [a-model-must-be-known-when-built.kav](a-model-must-be-known-when-built.kav) | A `Model` made from a value known only at run time is a compile error. | compile error |
| 8.3 | [tokens-are-for-limit-ai.kav](tokens-are-for-limit-ai.kav) | `tokens` in a limit rule counts what AI calls took: only for `limit Ai`, a compile error on another capability. | compile error |
| 8.4 | [model-has-no-temperature.kav](model-has-no-temperature.kav) | `Model` has no `temperature`: setting it is a compile error that says to use `effort`. | compile error |
| 8.4 | [a-model-that-always-thinks-refuses-thinking-off.kav](a-model-that-always-thinks-refuses-thinking-off.kav) | `thinking: false` on a model that always thinks first is a compile error. | compile error |
| 8.4 | [thinking-off-goes-up-to-high-effort.kav](thinking-off-goes-up-to-high-effort.kav) | `thinking: false` with `effort: XHigh` or `Max` is a compile error. | compile error |
| 8.5 | [agent-settings-change-with-with.kav](agent-settings-change-with-with.kav) | An agent's `settings` are an `AgentSettings`; a function shares them, and `with { … }` changes fields for one agent. | runs |
| 8.5 | [agent-settings-are-known-when-built.kav](agent-settings-are-known-when-built.kav) | An agent's `settings` are worked out when the program is built. | compile error |
| 8.3, 8.5 | [asks-under-way-is-known-when-built.kav](asks-under-way-is-known-when-built.kav) | `asks_under_way` is an `UnderWay` variant written in the settings; a value known only at run time is a compile error. Each way's behaviour is in `tools/cli/tests/token_budget.rs`, which needs a fake model. | compile error |
| 8.6 | [audit-log-that-cant-be-written-refuses.kav](audit-log-that-cant-be-written-refuses.kav) | An audit log that can't be written refuses the agent's calls: the call whose entry failed has run, and the next one is refused with `Denied`. | runs |
| 3.4, 8.1 | [function-types-limit-what-functions-use.kav](function-types-limit-what-functions-use.kav) | With `uses` in a function type, a function value may use only what the type lists. | compile error |
| 8.1 | [function-values-carry-their-makers-rights.kav](function-values-carry-their-makers-rights.kav) | The code that makes a function value pays for what it uses; a pure function can call what it is given. | runs |
