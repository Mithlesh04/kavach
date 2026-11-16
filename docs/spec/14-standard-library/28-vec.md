# 14.28 std.vec

Vectors, matrices and quaternions for games, graphics and simulations ([decision 0115](../../decisions/0115-generic-math-and-std-vec.md)), written in Kavach over number types (2.9). The program chooses the number type: `Vec3<Float32>`, as GPUs and game engines use, `Vec3<Float>`, or `Vec2<Int>` for a grid. Each is compiled for its type, so it runs as code written for that type would.

```kavach
use std.vec

const up = Vec3(x: 0.0, y: 1.0, z: 0.0)
const p = Vec3(x: 3.0, y: 4.0, z: 0.0)
print(p.length())                              // 5.0
print(p.cross(up))                             // Vec3(x: 0.0, y: 0.0, z: 3.0)
const turn = Quat.around(up, math.pi / 2.0)
const moved = Mat4.translation(Vec3(x: 1.0, y: 0.0, z: 0.0)) @ Vec4(x: 0.0, y: 0.0, z: 0.0, w: 1.0)
```

`Vec2`, `Vec3` and `Vec4` are records of `x`, `y`, `z` and `w`, as many as their number says, over any number type. `length`, `normalized`, `distance` and `lerp` need a `Float` or `Float32` (`where T: Floating`, 2.9); the others work on whole numbers too:

```kavach
const cell: Vec2<Int> = Vec2(x: 3, y: 4)
print(cell + Vec2(x: 1, y: 0))     // Vec2(x: 4, y: 4)
print(cell.length_squared())       // 25
cell.length()                      // error: `length` needs `T: Floating`, and `T` is `Int` here
```

| Operator or method | Gives |
|---|---|
| `a + b`, `a - b`, `-a` | Each part added, taken away, or turned negative |
| `a * k`, `a / k` | Each part times, or divided by, the number `k` |
| `a.scale(b)` | Each part times the same part of `b` |
| `a.dot(b)` | The dot product: the parts multiplied in pairs and added |
| `a.length()`, `a.length_squared()` | How long it is, and that squared, which needs no square root |
| `a.normalized()` | The same direction with length 1; a vector of length 0 stays as it is |
| `a.distance(b)` | How far `b` is from `a` |
| `a.lerp(b, t)` | The point `t` of the way from `a` to `b`: `a` at 0, `b` at 1 |
| `a.cross(b)` | For `Vec3`: the cross product, at right angles to both |

`Mat4<T>` is a 4 × 4 matrix of four columns, `c0` to `c3`, each a `Vec4`, as GPUs lay it out:

| Function, operator or method | Gives |
|---|---|
| `Mat4.identity()` | The matrix that changes nothing |
| `Mat4.translation(v)`, `Mat4.scaling(v)` | Moving by the `Vec3` `v`, and scaling each axis by its part of `v` |
| `Mat4.rotation_x(a)`, `rotation_y(a)`, `rotation_z(a)` | Turning by `a` radians around an axis, counterclockwise looking down it |
| `Mat4.perspective(fov_y, aspect, near, far)` | A perspective projection with depth from 0 to 1, as GPUs take it (15.3): `fov_y` the view's height in radians, `aspect` its width over its height |
| `Mat4.look_at(eye, target, up)` | A view from `eye` towards `target`, `up` pointing up |
| `m @ v` | The `Vec4` `v` transformed by `m` |
| `a.product(b)` | The matrix product: `b` then `a`, so `a.product(b) @ v` is `a @ (b @ v)` |
| `m.transposed()` | Rows and columns swapped |
| `m.mat3()` | The `Mat3` of its first three rows and columns: its turning and scaling, without its moving |

`Mat3<T>` is a 3 × 3 matrix of three columns, `c0` to `c2`, each a `Vec3`: turning and scaling without moving, as for normals, or a 2D transform:

| Function, operator or method | Gives |
|---|---|
| `Mat3.identity()` | The matrix that changes nothing |
| `Mat3.scaling(v)` | Each axis scaled by its part of the `Vec3` `v` |
| `Mat3.rotation_x(a)`, `rotation_y(a)`, `rotation_z(a)` | Turning by `a` radians around an axis, as `Mat4`'s |
| `m @ v` | The `Vec3` `v` transformed by `m` |
| `a.product(b)`, `m.transposed()` | As `Mat4`'s |
| `m.determinant()` | How much `m` scales volume; 0 when it flattens space |
| `m.inverse()` | The matrix that undoes `m`, or `none` when its determinant is 0 |

```kavach
const model = Mat4.translation(Vec3(x: 5.0, y: 0.0, z: 0.0)).product(Mat4.scaling(Vec3(x: 2.0, y: 2.0, z: 2.0)))
const undo = model.mat3().inverse() ?? Mat3.identity()
const normals = undo.transposed()      // for lighting: the moving is dropped
```

`Quat<T>` is a rotation as a quaternion, `x`, `y`, `z` and `w`:

| Function, operator or method | Gives |
|---|---|
| `Quat.identity()` | No rotation |
| `Quat.around(axis, angle)` | Turning by `angle` radians around the `Vec3` `axis`, which is normalized first |
| `a * b` | `b`, then `a` |
| `q.rotate(v)` | The `Vec3` `v` turned by `q` |
| `q.normalized()` | `q` with length 1, as rounding can drift it from 1 |
| `q.matrix()` | The same rotation as a `Mat4` |

- The angles are in radians, as `std.math`'s are (14.5).
- A matrix's or quaternion's `T` is `Float` or `Float32` (2.9). A literal gives the type: `Vec2(x: 1.0, y: 2.0)` is a `Vec2<Float>`, and `Vec2(x: 1, y: 2)` a `Vec2<Int>`.
- In a `gpu fn` (15.1), `Vec2` to `Vec4`, `Mat3` and `Mat4` of `Float32` are the GPU's own vector and matrix types, and their operators and methods its own operations. A `Quat` is a record of four parts there. A vertex fn's `position` and a fragment fn's color may be a `Vec4<Float32>` (15.3).

## Open questions

- SIMD registers for `Vec4<Float32>` and `Mat4<Float32>` on the CPU.
