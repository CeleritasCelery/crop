use crop::Rope;

mod common;

use common::{LARGE, MEDIUM, SMALL, TINY};

#[test]
fn byte() {
    for s in ["", "Hi", "Hello", "🐕‍🦺", TINY, SMALL, MEDIUM] {
        let r = Rope::from(s);
        for byte_idx in 0..s.len() {
            let r_byte = r.byte(byte_idx);
            let s_byte = s.as_bytes()[byte_idx];
            assert_eq!(r_byte, s_byte);
        }
    }
}

#[test]
fn empty_rope() {
    let r = Rope::new();
    assert!(r.is_empty());

    let r = Rope::from("");
    assert!(r.is_empty());
}

#[test]
fn trait_debug() {
    let s = "A string with \"quotes\" and\ttabs and escaped \
             escapes\\and\nnewlines\r\nand Unicode \u{2122} and emojis 😋";

    let r = Rope::from(s);

    assert_eq!(format!("Rope({s:?})"), format!("{r:?}"));
}

#[test]
fn trait_partial_eq() {
    for s in ["This is a service dog: 🐕‍🦺", TINY, SMALL, MEDIUM, LARGE]
    {
        let r = Rope::from(s);

        assert_eq!(r, r);
        assert_eq!(r.byte_slice(..), r.byte_slice(..));

        assert_eq!(r, s);
        assert_eq!(r.byte_slice(..), s);
        assert_eq!(r, r.byte_slice(..));
    }
}
