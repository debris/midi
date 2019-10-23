#![no_std]

use core::convert::TryInto;

#[derive(Debug)]
pub enum Error {
    HeaderType,
    HeaderLength,
    HeaderFormat,
    HeaderTracks,
    HeaderDivision,
}

/// The MIDI file format
#[derive(Debug)]
pub enum Format {
    Single,
    MultiTrack,
    MultiSequence,
}

pub struct Track;

pub struct Tracks<'a> {
    /// Pointer to the underlying unread midi data
    midi: &'a [u8],
    /// Number of tracks that have been already read
    tracks_read: u16,
    /// Number of tracks in the midi file
    tracks_expected: u16, 
}

impl<'a> Iterator for Tracks<'a> {
    type Item = Result<Track, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

pub struct MidiReader<'a> {
    pub format: Format,
    pub tracks: Tracks<'a>,
    pub division: u16,
}

/// Safely reads bytes from the slice
fn read(bytes: &[u8], pos: usize, len: usize) -> Option<&[u8]> {
    if pos + len > bytes.len() {
        return None
    }

    Some(&bytes[pos..pos + len])
}

/// Safely reads u16 from the slice
fn read_u16(bytes: &[u8], pos: usize) -> Option<u16> {
    read(bytes, pos, 2)
        .and_then(|data| data.try_into().ok())
        .map(|data| u16::from_le_bytes(data))
}

/// Safely reads u32 from the slice
fn read_u32(bytes: &[u8], pos: usize) -> Option<u32> {
    read(bytes, pos, 4)
        .and_then(|data| data.try_into().ok())
        .map(|data| u32::from_le_bytes(data))
}

impl<'a> MidiReader<'a> {
    pub fn new(midi: &'a [u8]) -> Result<Self, Error> {
        // validate header type
        if !midi.starts_with(b"MThd") {
            return Err(Error::HeaderType);
        }

        // validate header length
        let _ = read_u32(midi, 4)
            .filter(|length| *length == 6)
            .ok_or_else(|| Error::HeaderLength)?;

        // read format
        let format = read_u16(midi, 8)
            .and_then(|format| match format {
                0 => Some(Format::Single),
                1 => Some(Format::MultiTrack),
                2 => Some(Format::MultiSequence),
                _ => None,
            })
            .ok_or_else(|| Error::HeaderFormat)?;

        // read tracks
        let tracks = read_u16(midi, 10).ok_or_else(|| Error::HeaderTracks)?;
        let division = read_u16(midi, 12).ok_or_else(|| Error::HeaderDivision)?;


        let midi_reader = MidiReader {
            format,
            tracks: Tracks {
                midi: &midi[14..],
                tracks_read: 0,
                tracks_expected: tracks,
            },
            division,
        };

        Ok(midi_reader)
    }
}
