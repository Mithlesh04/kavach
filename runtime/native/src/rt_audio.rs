//! `std.audio` in native code (spec 14.27, decision 0114), from
//! `kavach_runtime_audio`, as the interpreter does: sounds and voices by
//! number. An entry of its own, so only a program with sound links
//! `cpal` (decision 0024).

use crate::heap::{alloc_slots, release};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use crate::shapes::items_of;
use kavach_kir::{RtOp, Shape};
use kavach_runtime_audio as audio;

/// Leaves `why` as an `AudioError` of `variant` for `kv_take_error`.
fn fail(variant: &str, why: &str) -> i64 {
    // The union is found by its first variant.
    if let Some(ty) = crate::shapes::find_union("AudioError", "Unavailable") {
        let text = crate::shapes::new_text(why, crate::heap::first_of(&Shape::Str));
        set_error(crate::rt_json::new_variant(ty, variant, &[text as u64]));
    }
    0
}

/// The floats in list `p` (owned) of `Float`s.
fn floats(p: i64) -> Vec<f64> {
    let all = items_of(p).iter().map(|s| f64::from_bits(*s)).collect();
    release(p);
    all
}

// `a` is a sound's or a voice's number, or what a sound is made from;
// `b` and `c` as `system.ops` says for each op.
crate::rt_data::entry!(kv_rt_audio, |op, a, b, c, code, _line| {
    match op {
        RtOp::AudioSound => {
            let samples = items_of(a)
                .iter()
                .map(|s| f32::from_bits(*s as u32))
                .collect();
            release(a);
            let fit = |v: i64, top: i64| v.clamp(1, top) as u32;
            audio::sound(samples, fit(b, 384_000), fit(c, 2) as u16)
        }
        RtOp::AudioDecode => {
            let made = crate::rt_bytes::with_bytes(a, audio::decode);
            release(a);
            made.unwrap_or_else(|why| fail("BadSound", &why))
        }
        RtOp::AudioInfo => {
            let (rate, channels, frames) = audio::info(a);
            alloc_slots(
                type_of(code),
                0,
                vec![rate as u64, channels as u64, frames as u64],
            )
        }
        RtOp::AudioDropSound => {
            audio::drop_sound(a);
            0
        }
        RtOp::AudioPlay => {
            let settings = floats(b);
            let part = |k: usize| settings.get(k).copied().unwrap_or(0.0);
            audio::play(a, part(0), part(1), c != 0).unwrap_or_else(|why| fail("Unavailable", &why))
        }
        RtOp::AudioVoice => {
            let settings = floats(a);
            let part = |k: usize| settings.get(k).copied().unwrap_or(0.0);
            audio::voice(part(0), part(1)).unwrap_or_else(|why| fail("Unavailable", &why))
        }
        RtOp::AudioVoiceRoom => audio::voice_room(a),
        RtOp::AudioVoiceFeed => {
            let samples: Vec<f32> = items_of(b)
                .iter()
                .map(|s| f32::from_bits(*s as u32))
                .collect();
            release(b);
            audio::voice_feed(a, &samples);
            0
        }
        RtOp::AudioStream => {
            let path = crate::shapes::str_of(a);
            release(a);
            let settings = floats(b);
            let part = |k: usize| settings.get(k).copied().unwrap_or(0.0);
            if !crate::rt_scopes::allowed(&path) {
                return fail(
                    "BadSound",
                    &format!("{path} is outside the folders this code may read"),
                );
            }
            match audio::stream(&path, part(0), part(1), c != 0) {
                Ok(v) => v,
                Err(Ok(why)) => fail("BadSound", &why),
                Err(Err(why)) => fail("Unavailable", &why),
            }
        }
        RtOp::AudioStop => {
            audio::stop(a);
            0
        }
        RtOp::AudioSet => {
            let value = floats(b).first().copied().unwrap_or(0.0);
            if c == 0 {
                audio::set_volume(a, value);
            } else {
                audio::set_pan(a, value);
            }
            0
        }
        RtOp::AudioListen => audio::listen().unwrap_or_else(|why| fail("Unavailable", &why)),
        RtOp::AudioTake => audio::take(a),
        RtOp::AudioStopListening => {
            audio::stop_listening(a);
            0
        }
        RtOp::AudioDropRecording => {
            audio::drop_recording(a);
            0
        }
        RtOp::AudioRecord => {
            audio::record(f64::from_bits(a as u64)).unwrap_or_else(|why| fail("Unavailable", &why))
        }
        _ => i64::from(audio::playing(a)),
    }
});
