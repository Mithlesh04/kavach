//! PNG written and read back, JPEG files of a known color read, and what
//! isn't an image refused.

use crate::{Image, decode, png};

#[test]
fn a_png_written_reads_back_the_same() {
    let rgba = vec![255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 9, 9, 9, 255];
    let file = png(2, 2, &rgba).expect("written");
    assert!(file.starts_with(b"\x89PNG"));
    assert_eq!(
        decode(&file).expect("read"),
        Image {
            width: 2,
            height: 2,
            rgba
        }
    );
}

#[test]
fn what_is_not_an_image_is_refused() {
    assert!(decode(b"hello").unwrap_err().contains("not a PNG or JPEG"));
    assert!(
        decode(b"\x89PNG\r\n\x1a\nbroken")
            .unwrap_err()
            .contains("PNG file can't be read")
    );
    assert!(
        decode(&[0xFF, 0xD8, 0xFF, 0x00])
            .unwrap_err()
            .contains("JPEG file can't be read")
    );
    assert!(png(2, 2, &[0; 3]).is_err());
}

/// 8 × 8 pixels of one color, made with ImageMagick: `red.jpg` of
/// rgb(200, 40, 40), and `grey.jpg`, a one-channel JPEG of 50% grey.
#[test]
fn a_jpeg_is_read_as_rgba() {
    let near = |a: u8, b: u8| a.abs_diff(b) <= 6;
    let red = decode(include_bytes!("red.jpg")).expect("read");
    assert_eq!((red.width, red.height, red.rgba.len()), (8, 8, 256));
    let [r, g, b, a] = red.rgba[..4] else {
        unreachable!()
    };
    assert!(
        near(r, 200) && near(g, 40) && near(b, 40) && a == 255,
        "{r} {g} {b} {a}"
    );
    let grey = decode(include_bytes!("grey.jpg")).expect("read");
    let [r, g, b, a] = grey.rgba[..4] else {
        unreachable!()
    };
    assert!(
        r == g && g == b && near(r, 128) && a == 255,
        "{r} {g} {b} {a}"
    );
}
