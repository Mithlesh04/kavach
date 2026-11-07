# 2.10 Where types must be written

Place | Type required? |
|---|---|
| Local variables | No, always inferred |
| Function parameters | **Yes** |
| Return type of a function that returns a value | **Yes** (no `->` means it returns nothing, 3.1) |
| Lambda parameters and return types | No, inferred from where the lambda is used |
| Record fields | **Yes** |

Signatures are the contract that people and AI tools read, so they are always written out, for private functions too. Everything inside a function body is inferred.
