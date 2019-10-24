#![no_std]

use core::convert::TryInto;

/// MIDI reading errors
#[derive(Debug)]
pub enum Error {
    HeaderType,
    HeaderLength,
    HeaderFormat,
    HeaderTracks,
    HeaderDivision,
    TrackType,
    TrackLength,
    TrackData,
    EventTime,
}

/// MIDI file format
#[derive(Debug)]
pub enum Format {
    Single,
    MultiTrack,
    MultiSequence,
}

#[derive(Debug)]
pub struct Event {
    pub delta_time: u32,
}

/// MIDI track chunk
pub struct Track<'a> {
    data: &'a [u8],
}

impl<'a> Iterator for Track<'a> {
    type Item = Result<Event, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        fn next_event(data: &[u8]) -> Result<(Event, usize), Error> {
            let (delta_time, bytes_read) = read_vlq(data)
                .ok_or_else(|| Error::EventTime)?;

            let event = Event {
                delta_time,
            };

            // TODO: parse events

            Ok((event, bytes_read))
        }

        if self.data.is_empty() {
            return None
        }

        let (event, bytes_read) = match next_event(self.data) {
            Ok(tuple) => tuple,
            Err(err) => return Some(Err(err)),
        };

        self.data = &self.data[bytes_read..];
        Some(Ok(event))
    }
}

/// Iterator over MIDI track chunks
#[derive(Debug, Clone)]
pub struct Tracks<'a> {
    /// Pointer to the underlying unread midi data
    midi: &'a [u8],
    /// Number of tracks that have been already read
    tracks_read: u16,
    /// Number of tracks in the midi file
    tracks_expected: u16, 
}

impl<'a> Iterator for Tracks<'a> {
    type Item = Result<Track<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // reads next track and returns it together with number of bytes read
        fn next_track(midi: &[u8]) -> Result<(Track, usize), Error> {
            // validate track type
            if !midi.starts_with(b"MTrk") {
                return Err(Error::TrackType)
            }

            // read track len
            let data_len = read_u32(&midi[4..])
                .ok_or_else(|| Error::TrackLength)?;

            // read data
            let data = read(&midi[8..], data_len as usize)
                .ok_or_else(|| Error::TrackData)?;

            let track = Track {
                data,
            };

            Ok((track, 8 + data.len()))
        }

        // exit if none more tracks are expected
        if self.tracks_read == self.tracks_expected {
            return None;
        }

        let (track, bytes_read) = match next_track(self.midi) {
            Ok(tuple) => tuple,
            Err(err) => return Some(Err(err)),
        };

        self.midi = &self.midi[bytes_read..];
        self.tracks_read += 1;

        Some(Ok(track))
    }
}

/// Lazy MIDI reader
pub struct MidiReader<'a> {
    pub format: Format,
    pub tracks: Tracks<'a>,
    pub division: u16,
}

/// Safely reads bytes from the slice
fn read(bytes: &[u8], len: usize) -> Option<&[u8]> {
    if len > bytes.len() {
        return None
    }

    Some(&bytes[..len])
}

/// Safely reads u16 from the slice
fn read_u16(bytes: &[u8]) -> Option<u16> {
    read(bytes, 2)
        .and_then(|data| data.try_into().ok())
        .map(|data| u16::from_le_bytes(data))
}

/// Safely reads u32 from the slice
fn read_u32(bytes: &[u8]) -> Option<u32> {
    read(bytes, 4)
        .and_then(|data| data.try_into().ok())
        .map(|data| u32::from_le_bytes(data))
}

/// Safely read variable-length quantity and returns it together with it's length in bytes
fn read_vlq(bytes: &[u8]) -> Option<(u32, usize)> {
    let mut result: u32 = 0;
    let mut size: usize = 0;
    while {
        // vlq must fit into 32 bit integer
        if size > 3 {
            return None;
        }

        let byte = read(&bytes[size..], 1)?[0];
        size += 1;
        result |= (byte & 0b0111_1111) as u32;
        (byte & 0b1000_0000) != 0
    } {
        result <<= 7;
    }
    Some((u32::from_le(result), size))
}

impl<'a> MidiReader<'a> {
    /// Creates new lazy MIDI reader
    pub fn new(midi: &'a [u8]) -> Result<Self, Error> {
        // validate header type
        if !midi.starts_with(b"MThd") {
            return Err(Error::HeaderType);
        }

        // validate header length
        let _ = read_u32(&midi[4..])
            .filter(|length| *length == 6)
            .ok_or_else(|| Error::HeaderLength)?;

        // read format
        let format = read_u16(&midi[8..])
            .and_then(|format| match format {
                0 => Some(Format::Single),
                1 => Some(Format::MultiTrack),
                2 => Some(Format::MultiSequence),
                _ => None,
            })
            .ok_or_else(|| Error::HeaderFormat)?;

        // read tracks
        let tracks = read_u16(&midi[10..]).ok_or_else(|| Error::HeaderTracks)?;
        let division = read_u16(&midi[12..]).ok_or_else(|| Error::HeaderDivision)?;


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

#[cfg(test)]
mod tests {
    use crate::read_vlq;

    #[test]
    fn test_read_vlq() {
        assert_eq!(Some((0, 1)), read_vlq(&[0]));
        assert_eq!(Some((1, 1)), read_vlq(&[1]));
        assert_eq!(Some((0x7f, 1)), read_vlq(&[0x7f]));
        assert_eq!(Some((0x80, 2)), read_vlq(&[0x81, 0x00]));
        assert_eq!(Some((0x3fff, 2)), read_vlq(&[0xff, 0x7f]));
        assert_eq!(Some((0x3e8, 2)), read_vlq(&[0x87, 0x68]));
        assert_eq!(Some((0xf4240, 3)), read_vlq(&[0xbd, 0x84, 0x40]));
    }
}
