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
