// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Sound files into samples (decision 0114): WAV, read here, and Ogg
//! Vorbis, through `lewton`. A file of more than two channels keeps its
//! first two.

use crate::mixer::SoundData;

/// The samples of a WAV or Ogg Vorbis file, told apart by their first
/// bytes; or why it can't be read.
pub fn decode(bytes: &[u8]) -> Result<SoundData, String> {
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        wav(bytes)
    } else if bytes.starts_with(b"OggS") {
        vorbis(bytes)
    } else {
        Err("this is not a WAV or Ogg Vorbis file".into())
    }
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// A WAV file: whole-number samples of 8, 16, 24 or 32 bits, or 32-bit
/// floats.
fn wav(bytes: &[u8]) -> Result<SoundData, String> {
    let bad = |why: &str| format!("this WAV file can't be read: {why}");
    let (mut format, mut channels, mut rate, mut bits) = (None, 0u16, 0u32, 0u16);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32_at(bytes, at + 4).ok_or_else(|| bad("a chunk is cut"))? as usize;
        let body = bytes
            .get(at + 8..(at + 8).saturating_add(size))
            .ok_or_else(|| bad("a chunk is cut"))?;
        if id == b"fmt " {
            let tag = u16_at(body, 0).ok_or_else(|| bad("its format is cut"))?;
            channels = u16_at(body, 2).unwrap_or(0);
            rate = u32_at(body, 4).unwrap_or(0);
            bits = u16_at(body, 14).unwrap_or(0);
            // WAVE_FORMAT_EXTENSIBLE names the format in its sub-format.
            format = Some(if tag == 0xFFFE {
                u16_at(body, 24).unwrap_or(0)
            } else {
                tag
            });
        } else if id == b"data" {
            let format = format.ok_or_else(|| bad("its data comes before its format"))?;
            if channels == 0 || !(1_000..=384_000).contains(&rate) {
                return Err(bad("its channels or rate are wrong"));
            }
            let samples = samples(body, format, bits).ok_or_else(|| {
                bad(&format!(
                    "{bits}-bit samples of format {format} aren't supported"
                ))
            })?;
            return Ok(stereo_at_most(samples, channels, rate));
        }
        at += 8 + size + (size & 1);
    }
    Err(bad("it has no data"))
}

/// Raw WAV samples as floats from -1 to 1.
pub(crate) fn samples(body: &[u8], format: u16, bits: u16) -> Option<Vec<f32>> {
    Some(match (format, bits) {
        (1, 8) => body
            .iter()
            .map(|b| (f32::from(*b) - 128.0) / 128.0)
            .collect(),
        (1, 16) => (body.as_chunks::<2>().0.iter())
            .map(|c| f32::from(i16::from_le_bytes(*c)) / 32_768.0)
            .collect(),
        (1, 24) => (body.as_chunks::<3>().0.iter())
            .map(|c| (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0)
            .collect(),
        (1, 32) => (body.as_chunks::<4>().0.iter())
            .map(|c| i32::from_le_bytes(*c) as f32 / 2_147_483_648.0)
            .collect(),
        (3, 32) => (body.as_chunks::<4>().0.iter())
            .map(|c| f32::from_le_bytes(*c).clamp(-1.0, 1.0))
            .collect(),
        _ => return None,
    })
}

/// An Ogg Vorbis file.
fn vorbis(bytes: &[u8]) -> Result<SoundData, String> {
    let bad = |e: lewton::VorbisError| format!("this Ogg Vorbis file can't be read: {e}");
    let mut reader =
        lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes)).map_err(bad)?;
    let channels = u16::from(reader.ident_hdr.audio_channels);
    let rate = reader.ident_hdr.audio_sample_rate;
    let mut all = Vec::new();
    while let Some(packet) = reader.read_dec_packet_itl().map_err(bad)? {
        all.extend(packet.into_iter().map(|s| f32::from(s) / 32_768.0));
    }
    // The last packet is padded; the stream says how many frames it has.
    if let Some(frames) = reader.get_last_absgp() {
        let wanted = (frames as usize).saturating_mul(usize::from(channels.max(1)));
        all.truncate(wanted);
    }
    Ok(stereo_at_most(all, channels, rate))
}

/// The first two channels of interleaved `samples` of `channels`.
fn stereo_at_most(samples: Vec<f32>, channels: u16, rate: u32) -> SoundData {
    if channels <= 2 {
        let whole = samples.len() - samples.len() % usize::from(channels);
        let mut samples = samples;
        samples.truncate(whole);
        return SoundData {
            samples,
            channels,
            rate,
        };
    }
    let n = usize::from(channels);
    let samples = (samples.chunks_exact(n))
        .flat_map(|f| [f[0], f[1]])
        .collect();
    SoundData {
        samples,
        channels: 2,
        rate,
    }
}
