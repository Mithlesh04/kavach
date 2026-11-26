# 8.7 Secrets

Fields marked `secret` (2.11) are tracked by the compiler wherever their value goes. A secret value **must not** flow into:

- `print`, `debug` or string interpolation
- AI prompts (`ask`)
- error messages
- the audit log or any log

```kavach
const prompt = `Card ends with {customer.card_number}`   // compile error E0302
```

A secret can only be passed to code that is allowed to receive it: a parameter or field that is itself marked `secret`, such as `StripePayments.api_key`. To deliberately reveal one, write `reveal(value)`. This is visible in code review and listed by `kavach audit`.

**Anything made from a secret is secret** (a maintainer, 2026-11-26; *not built yet*: today only a secret field, a name declared from one and a few expressions are tracked). An expression with a secret part is secret, whatever it computes: a method's result (`card.upper()`, `card.len()`, `card == other`), a list, tuple, dict or record holding it, an `if` or `match` whose branch gives it, and text built from it. A variable or field given a secret, when it is made or later (`=` and `+=`), is secret from then on. Only `reveal(…)` makes a value public, so even a secret's length or a comparison with it is written out where it happens:

```kavach
print(customer.card.upper())                 // compile error: a secret
print([customer.card])                       // compile error: a list holding a secret
let last = ""
last = customer.card                         // `last` is secret from here
print(reveal(customer.card.len()))           // allowed, and visible in review
```

Encoding is covered too: `json.of` and `json.stringify` write a `secret` field as `"<secret>"`, as `print` does (2.12, [decision 0132](../../decisions/0132-json-for-every-value.md)), and so does `grpc.encode<T>`, which builds JSON before its Protocol Buffers. The secret's value never leaves this way; to send it, pass it to a `secret` parameter or `reveal` it. The agent journal (8.5) is the one deliberate exception: it holds secrets as state, not as a log.

```kavach
fn charge(secret card: String, amount: Decimal) -> Receipt ! PaymentError uses Net { … }
fn remember(text: String) { … }

const receipt = try charge(order.card, order.amount)   // fine: `card` is secret
remember(order.card)                                  // compile error: `text` isn't secret
remember(reveal(order.card))                          // allowed, and visible in review
```

Inside `charge`, `card` is a secret like the field it came from, and so is a variable set from a secret (`const c = order.card`). The audit log writes `<secret>` for an argument of a `secret` parameter.

A value read from the environment with `env(name)` is a secret too (10.13): keys and passwords usually come that way.

**Returning a secret.** A function that returns a secret says so in its result: `-> secret T` ([decision 0094](../../decisions/0094-secret-results.md)). A call to it gives a secret, like the field it came from. Returning a secret from a function whose result isn't `secret` is a compile error, so a helper can't turn a secret into plain text. Interface methods and the methods that implement them agree on it.

```kavach
fn database_url() -> secret String ! Error uses Env {
    try env("DATABASE_URL")
}

fn leak() -> String ! Error uses Env {
    try env("DATABASE_URL")              // compile error: the result isn't `secret`
}

fn main() ! Error uses Env {
    const url = try database_url()       // a secret
    print(`using {url}`)                 // compile error: `url` is secret
    print(`using {reveal(url)}`)         // allowed, and visible in review
}
```

A lambda can't return a secret: a function type has no way to say so, so a named function with `-> secret T` does it.
