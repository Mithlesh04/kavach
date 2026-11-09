# 2.11 Secret fields

A field marked `secret` holds sensitive data such as card numbers, passwords or tokens.

```kavach
type Customer {
    name: String
    secret card_number: String
}
```

Secret values must not flow into `print`, `debug`, logs, AI prompts or error messages. The compiler rejects code that tries. Details are in [8.7](../08-capabilities-and-agents/07-secrets.md#87-secrets).
