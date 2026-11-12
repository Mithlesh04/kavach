# 8.7 Secrets

Fields marked `secret` (2.11) are tracked by the compiler wherever their value goes. A secret value **must not** flow into:

- `print`, `debug` or string interpolation
- AI prompts (`ask`)
- error messages
- the audit log or any log

```kavach
const prompt = "Card ends with {customer.card_number}"   // compile error E0302
```

A secret can only be passed to code that is allowed to receive it: a parameter or field that is itself marked `secret`, such as `StripePayments.api_key`. To deliberately reveal one, write `reveal(value)`. This is visible in code review and listed by `kavach audit`.

Encoding is covered too: `json.of` and `json.stringify` on a record with a `secret` field are a compile error (2.12), and when the value only becomes known through a generic function — `grpc.encode<T>` builds JSON before its Protocol Buffers — encoding **panics at run time** instead of writing the secret out. The agent journal (8.5) is the one deliberate exception: it holds secrets as state, not as a log.

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
    print("using {url}")                 // compile error: `url` is secret
    print("using {reveal(url)}")         // allowed, and visible in review
}
```

A lambda can't return a secret: a function type has no way to say so, so a named function with `-> secret T` does it.
