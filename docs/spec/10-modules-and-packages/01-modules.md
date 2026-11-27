# 10.1 Modules

- **One file is one module.** `orders.kav` is the module `orders`.
- **A folder groups modules.** `src/billing/invoice.kav` is the module `billing.invoice`.
- Everything is **private to its module** unless marked. `pub` lets the package's other modules use a name; `pub(out)` lets other packages use it too: the names a library offers ([decision 0127](../../decisions/0127-pub-out-for-what-a-package-offers.md)).
- **`pub(out)`** goes before `fn`, `type`, `const`, `interface`, `capability`, `agent`, `literal` and `use`. Only a library (`src/lib.kav`, 10.3) and its modules may write it: in an application it is a compile error, as no other package uses an application. A package may use only the `pub(out)` names of another; naming a plain `pub` one is a compile error that says so. The standard library offers all its `pub` names.
- **Unused names** (1.4 has the rule for variables): a function, type or module constant that nothing uses gets a warning. A private one is unused when nothing in its module uses it; a `pub` one when nothing in its package does ("`tokenize` is `pub`, but nothing in this package uses it: remove it, or make it `pub(out)`", the second half only in a library); a `pub(out)` one never is. `main` and names starting with `_` get none, nor do the functions of an application's entry file that has no `main`: they are there to be tested (11). Nor do the names of a file a Kavach tool made, such as the register blocks of `kavach svd` (12.1).
- A **method** without `pub` can be called anywhere in its own package, but not from another package, on a value (`o.tax()`) or on the type (`Order.blank()`). So a package's modules can share helper methods without making them part of what it offers. The `pub` methods of a `pub(out)` type are offered to other packages too; fields stay `pub` or private. A method an interface asks for (2.5) can be called wherever the interface can, `pub` or not.
- **`pub(read)`** before a record's field: other modules read it, but only the module that declares its type changes it, so that module keeps the type's invariants. Elsewhere, assigning it, changing it in place (`a.history.push(x)`), passing it to an `inout` parameter, setting it with `with`, or making a record of the type is a compile error that says so. Its own module, and an `extend` of the type in its package, use it as any field.

  ```kavach
  pub type Account {
      pub(read) balance: Decimal     // others read it; only this module changes it
      pub owner: String
  }
  ```

- A record with a private field can be **created only in its own module**. Other modules call a `pub fn` there that makes one, so the module keeps control of what its private fields hold. Reading one from outside data counts as creating it: `json.decode<T>`, `cli.parse<T>` or `ask<T>` of such a type, or of a type that holds one, is a compile error outside its module (2.12), and so is one with a `pub(read)` field.
- A module constant marked `pub` (`pub const width = 640`, 1.13) is read from other modules as `module.width`, or named in `use`; one without `pub` stays in its module. Files other than the entry file may have module constants, but no other statements.
- An `extend` of a type (2.5) in another module of the **same package** counts as the type's own module: it may read, change and build the type's private fields, so a type's methods can be spread over files without making its fields `pub`. An `extend` in another package sees only `pub` fields.

```kavach
pub type Invoice {
    pub id: Int
    pub total: Decimal
    internal_note: String          // private field
}

pub fn create(order: Order) -> Invoice { … }
fn round_total(x: Decimal) -> Decimal { … }    // private
```

```kavach
// src/lib.kav of the library `invoices`
pub(out) fn parse(text: String) -> Invoice { … }   // other packages too: never "unused"
pub(out) const version = "1.2"
pub fn tokenize(text: String) -> [Token] { … }     // this package: warned when it uses none
fn helper() { … }                                  // this module: warned when unused
```
