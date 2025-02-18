#![cfg(test)]

use crate::game::timing::{get_ron_options, Timing};
use ron::extensions::Extensions;
use ron::Options;
use serde::{Deserialize, Serialize};
use crate::game::beatmap::file::de_from_ron;

fn check_timing_eq(a: &Timing, b: &Timing) {
    assert_eq!(a.set_bpm, b.set_bpm);
    assert_eq!(a.set_speed, b.set_speed);
    assert_eq!(a.offset, b.offset);
}

#[test]
fn test_ser() {
    let mut vec = Vec::new();
    let t = Timing::default();
    t.serialize(
        &mut ron::Serializer::with_options(
            &mut vec,
            None,
            get_ron_options(),
        )
        .unwrap(),
    )
    .unwrap();

    let result = String::from_utf8_lossy(&vec);

    let nt = de_from_ron(&vec).unwrap();
    check_timing_eq(&t, &nt);
    
    let ant = de_from_ron(b"(bpm: 6000,speed: 1.0,offset: 0,time_signature: 4)").unwrap();
    check_timing_eq(&t, &ant);
    let ant = de_from_ron(b"(bpm: (6000),speed: 1.0,offset: 0,time_signature: 4)").unwrap();
    check_timing_eq(&t, &ant);

    assert_eq!(
        "(bpm:6000,speed:1.0,offset:0,time_signature:4)",
        result
    );
}
