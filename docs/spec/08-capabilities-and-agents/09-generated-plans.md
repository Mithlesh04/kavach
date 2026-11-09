# 8.9 Generated plans (Phase 4)

Instead of calling tools freely, an AI model can write a short **Kavach plan**. The runtime compiles and checks it before running it:

```kavach
const plan = try Plan.compile(source, grant: [Payments.refund, Email.send])
print(plan.preview())     // lists every capability call the plan can make
try plan.run()            // still subject to the agent's limits and approvals
```

A plan that uses a capability outside `grant` fails to compile, so it never runs.
