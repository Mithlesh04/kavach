use super::*;

/// Something shaped like a program: repeats, with some noise.
fn sample() -> Vec<u8> {
    (0..200_000u32)
        .map(|i| (i % 251) as u8 ^ (i / 1000) as u8)
        .collect()
}

#[test]
fn each_method_gives_back_the_same_bytes() {
    let program = sample();
    for method in [Method::Deflate, Method::Xz] {
        let mut file = b"the starter".to_vec();
        file.extend(pack(&program, method).unwrap());
        let t = trailer(&file).expect("a trailer");
        assert_eq!(t.method, method);
        assert_eq!(t.unpacked_len, program.len() as u64);
        assert!(t.packed_len < program.len() as u64 / 2, "{method:?} packs");
        assert_eq!(unpack(&file).unwrap(), program, "{method:?}");
    }
}

#[test]
fn a_file_without_a_trailer_is_not_packed() {
    assert_eq!(
        trailer(b"an ordinary program, long enough for a trailer"),
        None
    );
    assert_eq!(trailer(b"short"), None);
}

#[test]
fn a_damaged_program_is_refused() {
    let mut file = pack(&sample(), Method::Deflate).unwrap();
    let flip = file.len() - TRAILER - 1;
    file[flip] ^= 0xff;
    assert!(unpack(&file).is_err());
}
