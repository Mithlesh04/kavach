# 10.1 Modules

- **One file is one module.** `orders.kav` is the module `orders`.
- **A folder groups modules.** `src/billing/invoice.kav` is the module `billing.invoice`.
- Everything is **private to its module** unless marked `pub`.
- A **method** without `pub` can be called anywhere in its own package, but not from another package, on a value (`o.tax()`) or on the type (`Order.blank()`). So a package's modules can share helper methods without making them part of what it offers. A method an interface asks for (2.5) can be called wherever the interface can, `pub` or not.
- A record with a private field can be **created only in its own module**. Other modules call a `pub fn` there that makes one, so the module keeps control of what its private fields hold.
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
