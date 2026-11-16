# 14.29 std.image

Pictures as values ([decision 0117](../../decisions/0117-images-png-and-jpeg.md)): read from PNG and JPEG files, written as PNG, and given to the GPU as they are. An image is data, so it needs no capability; reading its file needs `Fs.read`.

```kavach
use std.image
use std.fs
use std.gpu

const img = try image.decode(try fs.read_bytes("grass.png"))
print("{img.width} × {img.height}")
const grass = try gpu.texture(img.width, img.height, img.pixels)
try fs.write_bytes("copy.png", image.png(img))
```

| Function or method | Gives |
|---|---|
| `image.decode(bytes) -> Image ! ImageError` | The picture in a PNG or JPEG file's bytes, told apart by their first bytes |
| `image.new(width, height, pixels) -> Image` | A picture of `pixels`, as `Image`'s field below: four for each pixel, `width * height * 4` in all |
| `image.png(img) -> Bytes` | `img` as a PNG file |
| `img.pixel(x, y) -> (UInt8, UInt8, UInt8, UInt8)` | The pixel at `x` (0 left) and `y` (0 top): red, green, blue and opacity |

- An `Image` is a record of `width: Int`, `height: Int` and `pixels: [UInt8]`: four for each pixel, red, green, blue and opacity, row by row from the top. That is what `gpu.texture` takes (14.24).
- Every image is read as 8 bits a part, with opacity: a grey image's parts are the same, an image without opacity is fully opaque, and 16-bit parts keep their high 8 bits.
- `ImageError` is `BadImage(reason: String)`: the bytes aren't a PNG or JPEG file, or the file is cut or damaged.
- `image.new` with a width or height outside 1 to 65,535, or the wrong number of pixels, is a panic (a contract).

## Open questions

- GIF and WebP: later, with their own decision. Compressed GPU textures are `gpu.compressed_texture` (14.24), and fonts `std.font` (14.31).
- Resizing and other image operations: packages, written in Kavach over `Image`.
