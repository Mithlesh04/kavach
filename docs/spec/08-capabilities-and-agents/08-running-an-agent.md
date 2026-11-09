# 8.8 Running an agent

An agent is started from ordinary code by providing an implementation for each capability it uses:

```kavach
fn main() ! Error {
    const bot = RefundBot.start(
        Payments: StripePayments(api_key: try env("STRIPE_KEY")),
        Email: SmtpEmail(host: "smtp.example.com"),
    )
    try bot.request(order)      // handle one event
}
```

- `start` takes one implementation, named by its capability, for every declared capability in the agent's `can` (whole or per method). A missing one, an extra one, or a value whose type doesn't implement the capability is a compile error. Built-in capabilities (`Ai`, `Net`, …) need none.
- `start` gives a running agent. Each `on` handler is a method on it, with the handler's parameters, giving the handler's value, and it can always fail with an `Error` ([decision 0076](../../decisions/0076-every-send-to-an-agent-can-fail.md)): the handler's own error, a refused call, or, when the handler panics, `panicked: <message> at <file>:<line>`. The panic is printed and written to the audit log (8.6), and the agent goes on serving other events (5.6). The code that sent the event goes on with all its own values as they were, however many sends fail.
- While a handler runs, its capability calls, and those of every function it calls, go to the agent's implementations, checked against its limits and approvals.
