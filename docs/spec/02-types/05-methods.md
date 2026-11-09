# 2.5 Methods

Methods are declared inside the type body. `self` is the value the method is called on.

```kavach
type Rect {
    width: Float
    height: Float

    fn area(self) -> Float {
        self.width * self.height
    }

    /// A function without `self` is called on the type: Rect.square(2.0)
    fn square(size: Float) -> Rect {
        Rect(width: size, height: size)
    }
}
```

Use `extend` to add methods to a union, or to a type from another module:

```kavach
extend String {
    fn shout(self) -> String { self.upper() + "!" }
}
```
