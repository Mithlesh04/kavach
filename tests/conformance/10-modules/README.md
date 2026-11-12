# 10. Modules and packages

Conformance tests for spec chapter [10. Modules and packages](../../../docs/spec/10-modules-and-packages/README.md). Format: [../README.md](../README.md). Most tests are a small project: a folder with `kavach.toml` and `src/`; the header is in `src/main.kav`, and the other files of `src/` are its modules. A test that needs no other module is one `.kav` file.

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 10.2 | [modules-share-pub-names/](modules-share-pub-names/src/main.kav) | `use` brings in another module; its `pub` names are used through it, or directly. | runs |
| 10.1 | [private-names-stay-private/](private-names-stay-private/src/main.kav) | A name without `pub` can't be used from another module. | compile error |
| 10.1 | [private-fields-stay-private/](private-fields-stay-private/src/main.kav) | A record field without `pub` can't be used from another module. | compile error |
| 10.1 | [records-with-private-fields-stay-in-their-module/](records-with-private-fields-stay-in-their-module/src/main.kav) | A record with a private field can be created only in its own module. | compile error |
| 10.1, 2.5 | [an-extend-in-the-package-sees-private-fields/](an-extend-in-the-package-sees-private-fields/src/main.kav) | An `extend` of a type in another file of the type's own package may read, change and build its private fields. | runs |
| 10.3 | [only-the-entry-has-main/](only-the-entry-has-main/src/main.kav) | Only the entry file can have `main`. | compile error |
| 10.12 | [dependencies-come-from-a-folder/](dependencies-come-from-a-folder/src/main.kav) | A dependency named by `path` is a package whose `src/lib.kav` is the module of that name; inside it, `use shapes` names its own module first. | runs |
| 10.12 | [a-package-names-its-own-root/](a-package-names-its-own-root/src/main.kav) | Inside a library's own project, its package name names its root module, as where it is a dependency. | runs |
| 10.12 | [dependencies-use-only-granted-capabilities/](dependencies-use-only-granted-capabilities/src/main.kav) | A dependency may use only the capabilities its entry grants in `can`; one without `can` must be pure. | compile error |
| 10.12 | [granted-capabilities-let-a-dependency-act/](granted-capabilities-let-a-dependency-act/src/main.kav) | A dependency whose entry grants a capability in `can` may use it. | runs |
| 10.12 | [dependencies-of-dependencies-get-no-more/](dependencies-of-dependencies-get-no-more/src/main.kav) | A dependency's own dependencies get at most what it was granted, whatever its `kavach.toml` says. | compile error |
| 10.13 | [env-values-are-secret.kav](env-values-are-secret.kav) | A value read with `env(name)` is a secret: it can't be printed unless revealed. | compile error |
| 10.13 | [env-fails-when-a-variable-is-not-set.kav](env-fails-when-a-variable-is-not-set.kav) | `env(name)` fails when the variable is not set, and `reveal` lets a value from it be shown. | runs |
| 10.7 | [c-calls-need-unsafe/](c-calls-need-unsafe/src/main.kav) | A C function is called only inside `unsafe`. | compile error |
| 10.7 | [c-functions-take-c-types/](c-functions-take-c-types/src/main.kav) | A C function takes numbers, booleans, `Ptr`s and `String`s, and gives numbers, booleans, `Ptr`s or nothing. | compile error |
| 10.7 | [c-records-hold-c-types/](c-records-hold-c-types/src/main.kav) | A `Ptr` going to C points to a number, a `Bool`, a `Ptr`, or a record of only those. | compile error |
| 10.7 | [c-callbacks-are-named-functions/](c-callbacks-are-named-functions/src/main.kav) | A C callback is the name of a Kavach function that can't fail; a lambda or closure is a compile error. | compile error |
