# 2.4 Records

```kavach
type User {
    name: String
    email: String
    age: Int = 0          // default value
}

const asha = User(name: "Asha", email: "asha@example.com", age: 31)
```

- Records are created with **named arguments**. Fields with defaults may be left out.
- **A field that may be left out:** `nickname?: String` is short for `nickname: String? = none`. Its type is `String?` (2.7), and a record made without it holds `none`. A field written `nickname: String?` must still be given, perhaps as `none`. Writing both (`nickname?: String?`) is an error.

```kavach
type Triage {
    urgent: Bool
    summary?: String       // String?, none when left out
}

const t = Triage(urgent: true)
print(t.summary ?? "no summary")       // no summary
```
- Records are **values**: assigning or passing a record never shares it (see chapter 6).

### Updating a record

`with` creates a copy with some fields changed:

```kavach
const older = asha with { age: asha.age + 1 }
```

Through a `let` binding, fields can be assigned directly. This changes only that variable's value:

```kavach
let user = asha
user.age = 32       // `asha` is unchanged
```
