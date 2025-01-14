#![cfg(test)]

use crate::game::timing::Timing;
use serde::Serialize;

#[test]
fn test_ser() {
    let mut vec = Vec::new();
    Timing::default().serialize(&mut ron::Serializer::new(&mut vec, None).unwrap()).unwrap();
    assert_eq!("(bpm:(6000),offset:0,time_signature:4)", String::from_utf8_lossy(&vec));
}