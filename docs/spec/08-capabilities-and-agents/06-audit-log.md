# 8.6 Audit log

The runtime records every capability call an agent makes. Nothing in the code needs to remember to log it.

```
2026-11-02T10:14:07Z RefundBot request#8812 Payments.refund(order_id: 1042, amount: 7499.00)
  verdict: approval required (amount > 5_000) → approved by finance:meera at 10:31:55Z
  result: ok Receipt(id: "re_3Kx…")
```

An `ask` is a call too (`Ai.ask`), and its lines say which model answered and how many tokens it took, so what each agent costs can be read from its log ([decision 0056](../../decisions/0056-the-model-is-chosen-in-code.md)):

```
2026-11-04T22:40:12Z Reviewer request#3 ask("Review this clause: …")
  verdict: allowed
  model: claude-opus-5-5, tokens: 412 in, 96 out
  result: ok Verdict(risky: true, reason: "…")
```

Under `kavach run --dry-run` (8.5), no model is asked, and the line says which one a real run would use: `model: claude-opus-5-5 (dry run: no tokens)`. So each call's model can be checked before a run that costs.

With `Audit(format: AuditFormat.JsonLines)` in the agent's settings, each entry is one JSON object on a line of its own instead, for log shippers (Vector, Fluent Bit, the OpenTelemetry Collector's file receiver): `time`, `agent`, `request`, `call`, `shown`, `verdict`, then `model`, `input_tokens` and `output_tokens` for an `ask`, and `result`.

```
{"time":"2026-11-04T22:40:12Z","agent":"Reviewer","request":3,"call":"Ai.ask","shown":"ask(\"Review this clause: …\")","verdict":"allowed","model":"claude-opus-5-5","input_tokens":412,"output_tokens":96,"result":"ok Verdict(risky: true, reason: \"…\")"}
```

With `otlp: true` as well, and a collector in `[log.otlp]` (14.12), each entry also goes to that collector as an OpenTelemetry log record named `AgentAudit`, with the JSON line's fields as attributes (`agent`, `request`, `call`, `verdict`, `model`, `input_tokens`, `output_tokens`, `result`) and `WARN` for a refused call, `INFO` otherwise. It is sent in batches by the thread that sends `std.log` events, so a slow collector never holds up an agent, and the file stays the record that counts. `otlp: true` without a `[log.otlp]` section is an error.

The audit log is append-only. Secret values (8.7) appear as `<secret>`.

**An audit log that can't be written** (a full disk, a folder that went away) never lets an agent go on unrecorded ([decision 0073](../../decisions/0073-durable-agents-across-restarts.md)). An entry is written after its call, so the entry that fails is printed to standard error instead, with a line saying the audit log can't be written. From then on, each of the agent's calls first tries to write a line to the log, `audit log writable again (1 entry printed to standard error)`. Until that works, the call is refused with `Denied`, `the audit log can't be written: No space left on device (os error 28)`, without running, and the rest of the program goes on:

```kavach
const sent = bot.request(order) catch e => {
    print(e)        // Denied(reason: "the audit log can't be written: …")
    return
}
```

**A handler that panics** is stopped there; the audit log gets `RefundBot request panicked: index 5 is out of range: the list has 2 items at main.kav:7` (the agent's name, then the handler's), and the send fails with the same `panicked: …` (8.8). In a durable agent the event stays unfinished in its journal, so it runs again at the next start, until it is set aside (8.5). Each run that panics writes the line again, with the panic printed as before, and after a restart the `finished after a restart` line below too. A send that joined the event (8.3) fails with the same `panicked: …`.

**An event that finishes after a restart** has no sender waiting for it (8.3), so its outcome goes to the audit log, with the event's number, whether `start` waited for it or it went on after asking a person: `RefundBot finished after a restart: event #17 of request: ok Receipt(id: "r17")`, or `RefundBot finished after a restart: event #17 of request: error Denied(reason: "not approved by finance (amount > 5_000): duplicate")`. An error nothing read is also printed to standard error, so it is seen with the audit log off too: `error in an event run again after a restart: event #17 of request: Denied(reason: "not approved by finance (amount > 5_000): duplicate")`. A program that sent the same event again gets the outcome too, and then nothing is printed ([decision 0072](../../decisions/0072-approvals-that-wait-across-restarts.md)).
