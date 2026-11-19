// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The speakers (decision 0114): the system's default output, opened at
//! the first `play` on a thread of its own, which keeps the `cpal` stream.
//! The stream's callback mixes on the real-time thread. New voices reach
//! it through a bounded channel it only polls, and finished ones go back
//! through another, so it never allocates, locks or waits.

use crate::mixer::{Voice, mix};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Mutex, OnceLock};

/// The most voices playing at once; more are not played.
pub(crate) const MOST_VOICES: usize = 256;

/// The samples mixed at a time, at most: the callback's buffer goes
/// through a scratch buffer of this size, made before it starts.
const SCRATCH: usize = 16_384;

/// The open output: where new voices go, and where finished ones come
/// back to be freed off the real-time thread.
pub(crate) struct Output {
    pub(crate) add: SyncSender<Voice>,
    pub(crate) freed: Mutex<Receiver<Voice>>,
}

/// The output, opened once; or why there is none.
pub(crate) fn output() -> Result<&'static Output, String> {
    static OUTPUT: OnceLock<Result<Output, String>> = OnceLock::new();
    OUTPUT.get_or_init(open).as_ref().map_err(Clone::clone)
}

fn open() -> Result<Output, String> {
    let (add, voices_in) = sync_channel::<Voice>(MOST_VOICES);
    let (to_free, freed) = sync_channel::<Voice>(MOST_VOICES * 4);
    let (ready, wait) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("kavach-audio".into())
        .spawn(move || {
            match start(voices_in, to_free) {
                Ok(stream) => {
                    let _ = ready.send(Ok(()));
                    // The stream plays while this thread keeps it.
                    let _keep = stream;
                    loop {
                        std::thread::park();
                    }
                }
                Err(why) => {
                    let _ = ready.send(Err(why));
                }
            }
        })
        .map_err(|e| e.to_string())?;
    wait.recv().map_err(|e| e.to_string())??;
    Ok(Output {
        add,
        freed: Mutex::new(freed),
    })
}

/// Opens the default output and starts its stream.
fn start(voices_in: Receiver<Voice>, to_free: SyncSender<Voice>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("this machine has no sound output")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, config, voices_in, to_free),
        SampleFormat::I16 => build::<i16>(&device, config, voices_in, to_free),
        SampleFormat::U16 => build::<u16>(&device, config, voices_in, to_free),
        SampleFormat::I32 => build::<i32>(&device, config, voices_in, to_free),
        other => {
            return Err(format!(
                "the sound output's samples ({other}) aren't supported"
            ));
        }
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

/// A stream of samples of type `T`, mixed as floats.
fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: StreamConfig,
    voices_in: Receiver<Voice>,
    to_free: SyncSender<Voice>,
) -> Result<cpal::Stream, String> {
    let channels = usize::from(config.channels.max(1));
    let rate = config.sample_rate;
    let mut voices: Vec<Voice> = Vec::with_capacity(MOST_VOICES);
    let mut scratch = vec![0.0f32; SCRATCH - SCRATCH % channels];
    let callback = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
        while voices.len() < MOST_VOICES {
            match voices_in.try_recv() {
                Ok(v) => voices.push(v),
                Err(_) => break,
            }
        }
        for chunk in data.chunks_mut(scratch.len()) {
            let mixed = &mut scratch[..chunk.len()];
            mix(mixed, channels, rate, &mut voices, &mut |v| {
                // Freed on the program's side; if that is full, here.
                let _ = to_free.try_send(v);
            });
            for (d, s) in chunk.iter_mut().zip(mixed.iter()) {
                *d = T::from_sample(*s);
            }
        }
    };
    device
        .build_output_stream(config, callback, |_| {}, None)
        .map_err(|e| e.to_string())
}
