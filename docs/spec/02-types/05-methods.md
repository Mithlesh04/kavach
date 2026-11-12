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

To extend a generic type, name its parameters after `extend`, as a generic function does: `extend<S> App<S>`. Each name stands for the type's own parameter in its place, so a type's many methods can be spread over files. A name that isn't one of the type's parameters is an error.

```kavach
// app.kav
pub type App<S> {
    pub state: S
    pub routes: Int = 0
}

// routes.kav
use app.{App}

extend<S> App<S> {
    pub fn route(self) -> App<S> {
        self with { routes: self.routes + 1 }
    }

    pub fn state_of(self) -> S {
        self.state
    }
}
```
