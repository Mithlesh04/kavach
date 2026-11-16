//! `std.audio` in the interpreter (spec 14.27), from
//! `kavach_runtime_audio`, as native code does: sounds and voices by
//! number.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_checker::Prim;
use kavach_runtime_audio as audio;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `audio.sound_id`, `play_id` and the rest, with every argument given.
    pub(crate) fn audio_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => (*n).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            _ => 0,
        };
        let floats = |i: usize| -> Vec<f64> {
            match args.get(i) {
                Some(Value::List(items)) => (items.iter())
                    .map(|v| match v {
                        Value::Float(f) => *f,
                        Value::Float32(f) => f64::from(*f),
                        _ => 0.0,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };
        let number = |made: Result<i64, String>, me: &Self, variant: &str| match made {
            Ok(n) => Ok(Value::Int(i128::from(n), Prim::Int)),
            Err(why) => {
                let reason = me.str_value(&why);
                Err(Flow::Fail(me.builtin_variant(
                    "AudioError",
                    variant,
                    vec![("reason", reason)],
                )))
            }
        };
        let n = int(0);
        Ok(match name {
            "sound_id" => {
                let samples = floats(0).into_iter().map(|f| f as f32).collect();
                let fit = |v: i64, top: i64| v.clamp(1, top) as u32;
                let made = audio::sound(samples, fit(int(1), 384_000), fit(int(2), 2) as u16);
                Value::Int(i128::from(made), Prim::Int)
            }
            "decode_id" => {
                let made = match args.first() {
                    Some(Value::Bytes(b)) => audio::decode(b),
                    _ => Err("no bytes".into()),
                };
                return number(made, self, "BadSound");
            }
            "info_id" => {
                let (r, c, f) = audio::info(n);
                let all = [r, c, f].map(|v| Value::Int(i128::from(v), Prim::Int));
                Value::List(Rc::new(all.to_vec().into()))
            }
            "drop_sound_id" => {
                audio::drop_sound(n);
                Value::Unit
            }
            "play_id" => {
                let s = floats(1);
                let part = |k: usize| s.get(k).copied().unwrap_or(0.0);
                let looped = matches!(args.get(2), Some(Value::Bool(true)));
                return number(
                    audio::play(n, part(0), part(1), looped),
                    self,
                    "Unavailable",
                );
            }
            "stop_id" => {
                audio::stop(n);
                Value::Unit
            }
            "set_id" => {
                let v = floats(1).first().copied().unwrap_or(0.0);
                if int(2) == 0 {
                    audio::set_volume(n, v);
                } else {
                    audio::set_pan(n, v);
                }
                Value::Unit
            }
            "listen_id" => return number(audio::listen(), self, "Unavailable"),
            "take_id" => Value::Int(i128::from(audio::take(n)), Prim::Int),
            "stop_listening_id" => {
                audio::stop_listening(n);
                Value::Unit
            }
            "drop_recording_id" => {
                audio::drop_recording(n);
                Value::Unit
            }
            "record_id" => {
                let seconds = match args.first() {
                    Some(Value::Float(f)) => *f,
                    _ => 0.0,
                };
                return number(audio::record(seconds), self, "Unavailable");
            }
            _ => Value::Bool(audio::playing(n)),
        })
    }
}
