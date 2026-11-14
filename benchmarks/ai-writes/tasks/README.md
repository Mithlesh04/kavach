# benchmarks/ai-writes/tasks/

The benchmark's problems, each written once, apart from any language: what every language's version of the task must do.

| File | What it holds |
|---|---|
| `tasks.json` | Each task's `id`, its `kind` (basics, data, algorithms, kavach features) and its `description`, the words the model is given |

A language takes part in a task by having `languages/<language>/tasks/<id>.json` and a reference solution for it ([../languages/](../languages/README.md)).
