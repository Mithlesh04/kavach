//! Sound played from its file a piece at a time (`audio.stream`, spec
//! 14.27): long music without holding it whole in memory. A decoder
//! thread reads the file and fills a ring of about a second; the mixer's
//! voice takes frames from the ring, never waiting: if the ring is ever
//! empty it plays silence that moment.

use crate::input::Ring;
use crate::mixer::Controls;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// What the decoder gives the mixer: frames as left and right, and
/// whether it reached the end.
pub(crate) struct Stream {
    pub(crate) ring: Ring,
    pub(crate) rate: u32,
    pub(crate) ended: AtomicBool,
}

/// A file being read: WAV samples left in its data, or Ogg Vorbis.
enum Reader {
    Wav {
        file: BufReader<File>,
        format: u16,
        bits: u16,
        channels: u16,
        left: u64,
    },
    Vorbis(Box<lewton::inside_ogg::OggStreamReader<BufReader<File>>>),
}

/// Opens `path` for reading a piece at a time: the reader, its rate.
fn open(path: &str) -> Result<(Reader, u32), String> {
    let cant = |e: std::io::Error| format!("{path} can't be read: {e}");
    let mut file = BufReader::new(File::open(path).map_err(cant)?);
    let mut head = [0u8; 12];
    file.read_exact(&mut head).map_err(cant)?;
    file.seek(SeekFrom::Start(0)).map_err(cant)?;
    if &head[..4] == b"OggS" {
        let reader = lewton::inside_ogg::OggStreamReader::new(file)
            .map_err(|e| format!("{path} can't be read as Ogg Vorbis: {e}"))?;
        let rate = reader.ident_hdr.audio_sample_rate;
        return Ok((Reader::Vorbis(Box::new(reader)), rate));
    }
    if &head[..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err(format!("{path} is not a WAV or Ogg Vorbis file"));
    }
    wav_header(file, path)
}

/// Reads a WAV file's chunks up to its data: the reader there, its rate.
fn wav_header(mut file: BufReader<File>, path: &str) -> Result<(Reader, u32), String> {
    let bad = |why: &str| format!("{path} can't be read as WAV: {why}");
    file.seek(SeekFrom::Start(12))
        .map_err(|e| bad(&e.to_string()))?;
    let (mut format, mut channels, mut rate, mut bits) = (None, 0u16, 0u32, 0u16);
    loop {
        let mut chunk = [0u8; 8];
        file.read_exact(&mut chunk)
            .map_err(|_| bad("it has no data"))?;
        let size = u64::from(u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]));
        if &chunk[..4] == b"fmt " {
            let mut body = vec![0u8; size as usize];
            file.read_exact(&mut body)
                .map_err(|_| bad("its format is cut"))?;
            let at = |i: usize| {
                body.get(i..i + 2)
                    .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
            };
            channels = at(2);
            rate = u32::from(at(4)) | (u32::from(at(6)) << 16);
            bits = at(14);
            format = Some(if at(0) == 0xFFFE { at(24) } else { at(0) });
            if size & 1 == 1 {
                file.seek(SeekFrom::Current(1))
                    .map_err(|e| bad(&e.to_string()))?;
            }
        } else if &chunk[..4] == b"data" {
            let format = format.ok_or_else(|| bad("its data comes before its format"))?;
            if channels == 0 || !(1_000..=384_000).contains(&rate) {
                return Err(bad("its channels or rate are wrong"));
            }
            if crate::decode::samples(&[], format, bits).is_none() {
                return Err(bad(&format!(
                    "{bits}-bit samples of format {format} aren't supported"
                )));
            }
            let reader = Reader::Wav {
                file,
                format,
                bits,
                channels,
                left: size,
            };
            return Ok((reader, rate));
        } else {
            let skip = (size + (size & 1)) as i64;
            file.seek(SeekFrom::Current(skip))
                .map_err(|e| bad(&e.to_string()))?;
        }
    }
}

impl Reader {
    /// The next piece as left and right frames, interleaved; `None` at
    /// the end.
    fn next(&mut self) -> Option<Vec<f32>> {
        match self {
            Reader::Wav {
                file,
                format,
                bits,
                channels,
                left,
            } => {
                let frame = u64::from(*channels) * u64::from(*bits / 8);
                let want = (*left).min(4_096 * frame.max(1));
                if want == 0 {
                    return None;
                }
                let mut bytes = vec![0u8; want as usize];
                file.read_exact(&mut bytes).ok()?;
                *left -= want;
                let samples = crate::decode::samples(&bytes, *format, *bits)?;
                Some(stereo(&samples, *channels))
            }
            Reader::Vorbis(reader) => loop {
                let packet = reader.read_dec_packet_itl().ok()??;
                if packet.is_empty() {
                    continue;
                }
                let channels = u16::from(reader.ident_hdr.audio_channels);
                let samples: Vec<f32> = packet.iter().map(|s| f32::from(*s) / 32_768.0).collect();
                return Some(stereo(&samples, channels));
            },
        }
    }
}

/// Interleaved `samples` of `channels` as left and right.
fn stereo(samples: &[f32], channels: u16) -> Vec<f32> {
    let n = usize::from(channels.max(1));
    (samples.chunks_exact(n))
        .flat_map(|f| if n == 1 { [f[0], f[0]] } else { [f[0], f[1]] })
        .collect()
}

/// Starts reading `path` on a thread of its own, from its start again at
/// its end when `looped`, until `controls` says the voice is done.
pub(crate) fn start(
    path: &str,
    looped: bool,
    controls: Arc<Controls>,
) -> Result<Arc<Stream>, String> {
    let (reader, rate) = open(path)?;
    let stream = Arc::new(Stream {
        ring: Ring::new(rate as usize * 2),
        rate,
        ended: AtomicBool::new(false),
    });
    let (path, filled) = (path.to_string(), stream.clone());
    std::thread::Builder::new()
        .name("kavach-stream".into())
        .spawn(move || fill(reader, &path, looped, &filled, &controls))
        .map_err(|e| e.to_string())?;
    Ok(stream)
}

/// The decoder thread: pieces into the ring as it has room.
fn fill(mut reader: Reader, path: &str, looped: bool, stream: &Stream, controls: &Controls) {
    let gone = || controls.done.load(Ordering::Acquire) || controls.stop.load(Ordering::Relaxed);
    let mut pending: Vec<f32> = Vec::new();
    while !gone() {
        if pending.is_empty() {
            match reader.next() {
                Some(piece) => pending = piece,
                None if looped => match open(path) {
                    Ok((again, _)) => {
                        reader = again;
                        continue;
                    }
                    Err(_) => break,
                },
                None => break,
            }
        }
        let fits = stream.ring.room().min(pending.len()) & !1;
        if fits == 0 {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        stream.ring.write(pending.drain(..fits));
    }
    stream.ended.store(true, Ordering::Release);
}
