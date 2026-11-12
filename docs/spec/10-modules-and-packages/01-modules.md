# 10.1 Modules

- **One file is one module.** `orders.kav` is the module `orders`.
- **A folder groups modules.** `src/billing/invoice.kav` is the module `billing.invoice`.
- Everything is **private to its module** unless marked `pub`.
- A record with a private field can be **created only in its own module**. Other modules call a `pub fn` there that makes one, so the module keeps control of what its private fields hold.

```kavach
pub type Invoice {
    pub id: Int
    pub total: Decimal
    internal_note: String          // private field
}

pub fn create(order: Order) -> Invoice { … }
fn round_total(x: Decimal) -> Decimal { … }    // private
```
