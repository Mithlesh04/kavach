//! The microphone (spec 14.27): the system's default input, opened by each
//! `listen` on a thread of its own, which keeps the `cpal` stream until
//! the recording stops. The stream's callback writes into a ring made
//! before it starts (`Ring`), so it never allocates, locks or waits;
//! `take` empties the ring on the program's side.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// The most seconds kept between takes; later samples are dropped.
pub(crate) const KEPT_SECONDS: usize = 60;

/// Samples from one writer (the callback) to one reader (`take`), in a
/// buffer of fixed size: what doesn't fit is dropped.
pub(crate) struct Ring {
    samples: Vec<AtomicU32>,
    /// How many samples were written, and read, in all.
    written: AtomicUsize,
    read: AtomicUsize,
}

impl Ring {
    pub(crate) fn new(size: usize) -> Ring {
        Ring {
            samples: (0..size.max(1)).map(|_| AtomicU32::new(0)).collect(),
            written: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    /// Adds `samples` while there is room (the callback's side).
    pub(crate) fn write(&self, samples: impl Iterator<Item = f32>) {
        let size = self.samples.len();
        let read = self.read.load(Ordering::Acquire);
        let mut at = self.written.load(Ordering::Relaxed);
        for s in samples {
            if at - read >= size {
                break;
            }
            self.samples[at % size].store(s.to_bits(), Ordering::Relaxed);
            at += 1;
        }
        self.written.store(at, Ordering::Release);
    }

    /// Every sample written and not yet read (the program's side).
    pub(crate) fn take(&self) -> Vec<f32> {
        let size = self.samples.len();
        let written = self.written.load(Ordering::Acquire);
        let read = self.read.load(Ordering::Relaxed);
        let out = (read..written)
            .map(|i| f32::from_bits(self.samples[i % size].load(Ordering::Relaxed)))
            .collect();
        self.read.store(written, Ordering::Release);
        out
    }
}

/// A recording: its ring, rate and channels, and how to stop it.
pub(crate) struct Listening {
    pub(crate) ring: Arc<Ring>,
    pub(crate) rate: u32,
    pub(crate) channels: u16,
    stop: Arc<AtomicBool>,
    thread: std::thread::Thread,
}

impl Listening {
    /// Stops the stream; what it heard stays to be taken.
    pub(crate) fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        self.thread.unpark();
    }
}

/// Starts recording from the default input, or says why it can't.
pub(crate) fn listen() -> Result<Listening, String> {
    let (ready, wait) = std::sync::mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let handle = std::thread::Builder::new()
        .name("kavach-microphone".into())
        .spawn(move || match start() {
            Ok((stream, ring, rate, channels)) => {
                let _ = ready.send(Ok((ring, rate, channels)));
                // The stream records while this thread keeps it.
                while !stopped.load(Ordering::Acquire) {
                    std::thread::park();
                }
                drop(stream);
            }
            Err(why) => {
                let _ = ready.send(Err(why));
            }
        })
        .map_err(|e| e.to_string())?;
    let (ring, rate, channels) = wait.recv().map_err(|e| e.to_string())??;
    Ok(Listening {
        ring,
        rate,
        channels,
        stop,
        thread: handle.thread().clone(),
    })
}

/// Opens the default input and starts its stream, keeping at most two
/// channels and `KEPT_SECONDS` seconds.
fn start() -> Result<(cpal::Stream, Arc<Ring>, u32, u16), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("this machine has no microphone")?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let rate = config.sample_rate.clamp(1_000, 384_000);
    let channels = config.channels.clamp(1, 2);
    let ring = Arc::new(Ring::new(
        rate as usize * usize::from(channels) * KEPT_SECONDS,
    ));
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, config, ring.clone()),
        SampleFormat::I16 => build::<i16>(&device, config, ring.clone()),
        SampleFormat::U16 => build::<u16>(&device, config, ring.clone()),
        SampleFormat::I32 => build::<i32>(&device, config, ring.clone()),
        other => {
            return Err(format!(
                "the microphone's samples ({other}) aren't supported"
            ));
        }
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, ring, rate, channels))
}

/// A stream of samples of type `T`, kept as floats.
fn build<T: SizedSample>(
    device: &cpal::Device,
    config: StreamConfig,
    ring: Arc<Ring>,
) -> Result<cpal::Stream, String>
where
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels.max(1));
    let callback = move |data: &[T], _: &cpal::InputCallbackInfo| {
        ring.write(first_two(data, channels).map(|s| s.to_sample::<f32>()));
    };
    device
        .build_input_stream(config, callback, |_| {}, None)
        .map_err(|e| e.to_string())
}

/// The samples of the first two channels of `data`, which has `channels`
/// interleaved.
pub(crate) fn first_two<T: Copy>(data: &[T], channels: usize) -> impl Iterator<Item = T> + '_ {
    let kept = channels.min(2);
    data.chunks(channels.max(1))
        .flat_map(move |frame| frame.iter().take(kept).copied())
}
