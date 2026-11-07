# 8.2 Declaring your own capabilities

A `capability` names an outside system your code depends on:

```kavach
capability Payments {
    fn refund(order_id: Int, amount: Decimal) -> Receipt ! PaymentError
    fn charge(order_id: Int, amount: Decimal) -> Receipt ! PaymentError
}
```

Code calls it through its name:

```kavach
const receipt = try Payments.refund(order.id, order.amount)
```

An implementation is an ordinary type that implements the capability like an interface. Implementations need their own capabilities (here `Net`):

```kavach
type StripePayments: Payments {
    secret api_key: String

    fn refund(self, order_id: Int, amount: Decimal) -> Receipt ! PaymentError uses Net { … }
    fn charge(self, order_id: Int, amount: Decimal) -> Receipt ! PaymentError uses Net { … }
}
```

Capabilities can be granted whole (`Payments`) or per method (`Payments.refund`).

Every capability call can be refused (by a `limit`, an approval, or because nothing implements the capability here). So every capability method declares an error type, and that type is a union with the variant `Denied(reason: String)`; the compiler checks it. Writing it out keeps the refusal visible to anyone reading the code:

```kavach
type PaymentError = Declined | Unavailable | Denied(reason: String)

const receipt = Payments.refund(order.id, order.amount) catch e => match e {
    PaymentError.Denied(reason) => fail RefundError.Blocked(reason),
    PaymentError.Declined => fail RefundError.Declined,
    PaymentError.Unavailable => fail RefundError.TryLater,
}
```

A call is answered by the implementation the running agent was started with (8.8), or the one a test provides (11.8). Anywhere else nothing implements the capability, and the call fails with `Denied("no implementation of Payments")`.
