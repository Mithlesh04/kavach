# 14.16 std.agents

| Function | Gives |
|---|---|
| `agents.call_key() -> String?` | Inside a capability's implementation, called from a durable agent's handler (8.5): the key of the running call, the same in the run after a crash, such as `RefundBot-7-2` (the agent, the event's number, the call's place in it). `none` anywhere else |
| `agents.waiting() -> [ApprovalRequest]` | The approval requests (8.3) that wait for an answer in the project's agents folder (8.5), oldest first. Needs `Approvals` |
| `agents.answer(id, approved: Bool, by: String, reason: String? = none) ! ApprovalError` | Answers request `id`, as `kavach approve` and `kavach deny` do (10.5). `by` names the person who answers; it goes to the audit log. Needs `Approvals` |

A call that a crash cut off runs again after the restart, so it may run twice (8.5). A service that is told the key can drop the repeat, as Stripe and other payment APIs do with an `Idempotency-Key` header ([decision 0062](../../decisions/0062-crashes-restarts-and-clocks.md)):

```kavach
use std.agents

type Bank: Payments {
    fn refund(self, order_id: Int) -> Receipt ! PaymentError {
        // The bank keeps the keys it has seen and drops a second refund.
        const key = agents.call_key() ?? "order-{order_id}"
        try self.api.refund(order_id, key)
    }
}
```

Like reading the clock, `call_key` needs no capability.

## Answering approvals in code

`waiting` and `answer` let a program take a person's answer from a web page, a chat or an email, and pass it on ([decision 0072](../../decisions/0072-approvals-that-wait-across-restarts.md)). The program that answers may be another program than the one that waits: both look in the same project's agents folder.

```kavach
use std.agents

fn review(person: String) ! ApprovalError uses Approvals, Console {
    for r in agents.waiting() {
        print("{r.id}: {r.call}, for {r.by} until {r.until}")
    }
    try agents.answer("RefundBot-17-2", approved: true, by: person)
}
```

An `ApprovalRequest` has these fields, all `String`:

| Field | Holds |
|---|---|
| `id` | The request's number, `RefundBot-17-2` (8.3) |
| `agent_name` | The agent that asks, `RefundBot` |
| `call` | The call waiting, with secrets hidden: `Payments.refund(order_id: 17, amount: 6000, card: <secret>)` |
| `by` | The role asked, `finance` |
| `why` | The rule that asked: `amount > 5_000` |
| `asked`, `until` | When it was asked and when it runs out, in UTC: `2026-11-06T09:30:00Z` |

`answer` fails with `ApprovalError`:

- `ApprovalError.NotWaiting(id)` when no request `id` waits: it was never made, it was already answered, or it ran out of time.
- `ApprovalError.Failed(id, reason)` when the answer can't be written, such as `can't write the answer: Access is denied. (os error 5)`.

Rules:

1. **An agent can't approve its own requests.** `Approvals` may never be listed in an agent's `can`; that is a compile error, `an agent can't answer approvals: \`Approvals\` can't be in \`can\` (8.3)`. Only code outside agents answers.
2. **Under `kavach test` and in a dry run**, agents keep nothing (8.5), so `waiting()` is empty and `answer` fails with `NotWaiting`.
3. Who may answer as `finance` is the program's to check, with its own sign-in. Kavach writes the name it is given to the audit log.
