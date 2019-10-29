//! Low-level `SMF` reading interface.

use crate::{
    Action, Error, ErrorKind, Event, EventKind, Format, MetaEvent, MidiEvent, MidiEventKind,
    SysexEvent, Text,
};
use core::convert::TryInto;
use core::str;

fn context(context: &'static str) -> impl FnOnce(ErrorKind) -> Error {
    move |kind| Error { context, kind }
}

fn read_bytes<'a>(data: &mut &'a [u8], len: usize) -> Result<&'a [u8], ErrorKind> {
    if data.len() < len {
        return Err(ErrorKind::Fatal);
    }
    let (result, rest) = data.split_at(len);
    *data = rest;
    Ok(result)
}

fn read_u7(data: &mut &[u8]) -> Result<u8, ErrorKind> {
    let byte = read_u8(data)?;
    if byte <= 0x7f {
        Ok(byte)
    } else {
        Err(ErrorKind::Invalid)
    }
}

fn read_u8(data: &mut &[u8]) -> Result<u8, ErrorKind> {
    read_bytes(data, 1).map(|b| b[0]).map(u8::from_be)
}

fn read_u16(data: &mut &[u8]) -> Result<u16, ErrorKind> {
    read_bytes(data, 2)
        .map(|b| b.try_into().unwrap())
        .map(u16::from_be_bytes)
}

fn read_u24(data: &mut &[u8]) -> Result<u32, ErrorKind> {
    read_bytes(data, 3)
        .map(|b| {
            let mut bytes = [0u8; 4];
            bytes[1..].copy_from_slice(&b);
            bytes
        })
        .map(|b| b.try_into().unwrap())
        .map(u32::from_be_bytes)
}

fn read_u32(data: &mut &[u8]) -> Result<u32, ErrorKind> {
    read_bytes(data, 4)
        .map(|b| b.try_into().unwrap())
        .map(u32::from_be_bytes)
}

fn read_format(data: &mut &[u8]) -> Result<Format, ErrorKind> {
    let value = read_u16(data)?;
    let format = match value {
        0 => Format::Single,
        1 => Format::MultiTrack,
        2 => Format::MultiSequence,
        _ => return Err(ErrorKind::Invalid),
    };
    Ok(format)
}

fn expect_bytes(data: &mut &[u8], expected: &[u8]) -> Result<(), ErrorKind> {
    if read_bytes(data, expected.len())? != expected {
        return Err(ErrorKind::Invalid);
    }
    Ok(())
}

fn expect_u8(data: &mut &[u8], expected: u8) -> Result<(), ErrorKind> {
    if read_u8(data)? != expected {
        return Err(ErrorKind::Invalid);
    }
    Ok(())
}

fn expect_u32(data: &mut &[u8], expected: u32) -> Result<(), ErrorKind> {
    if read_u32(data)? != expected {
        return Err(ErrorKind::Invalid);
    }
    Ok(())
}

fn read_vlq(data: &mut &[u8]) -> Result<u32, ErrorKind> {
    let mut result: u32 = 0;
    let mut size: usize = 0;
    while {
        // vlq must fit into 32 bit integer
        if size > 3 {
            return Err(ErrorKind::Invalid);
        }

        let byte = read_u8(data)?;
        size += 1;
        result |= (byte & 0b0111_1111) as u32;
        (byte & 0b1000_0000) != 0
    } {
        result <<= 7;
    }

    Ok(result)
}

fn read_data<'a>(data: &mut &'a [u8]) -> Result<&'a [u8], ErrorKind> {
    let length = read_vlq(data)?;
    read_bytes(data, length as usize)
}

fn read_text<'a>(data: &mut &'a [u8]) -> Result<Text<'a>, ErrorKind> {
    read_data(data).map(Text::new)
}

fn read_action(data: &mut &[u8]) -> Result<Action, ErrorKind> {
    let byte = read_u8(data)?;
    let action = match byte {
        0x00 => Action::Disconnect,
        0x7f => Action::Reconnect,
        _ => return Err(ErrorKind::Invalid),
    };

    Ok(action)
}

fn read_meta_event<'a>(bytes: &mut &'a [u8]) -> Result<MetaEvent<'a>, ErrorKind> {
    let meta_type = read_u8(bytes)?;
    let meta_event = match meta_type {
        0x00 => {
            expect_u8(bytes, 2)?;
            let number = read_u16(bytes)?;
            MetaEvent::SequenceNumber(number)
        }
        0x01 => read_text(bytes).map(MetaEvent::Text)?,
        0x02 => read_text(bytes).map(MetaEvent::CopyrightNotice)?,
        0x03 => read_text(bytes).map(MetaEvent::Name)?,
        0x04 => read_text(bytes).map(MetaEvent::InstrumentName)?,
        0x05 => read_text(bytes).map(MetaEvent::Lyric)?,
        0x06 => read_text(bytes).map(MetaEvent::Marker)?,
        0x07 => read_text(bytes).map(MetaEvent::CuePoint)?,
        0x20 => {
            expect_u8(bytes, 1)?;
            let channel = read_u8(bytes)?;
            MetaEvent::ChannelPrefix(channel)
        }
        0x2f => {
            expect_u8(bytes, 0)?;
            MetaEvent::EndOfTrack
        }
        0x51 => {
            expect_u8(bytes, 3)?;
            let tempo = read_u24(bytes)?;
            MetaEvent::SetTempo(tempo)
        }
        0x54 => {
            expect_u8(bytes, 5)?;
            let hh = read_u8(bytes)?;
            let mm = read_u8(bytes)?;
            let ss = read_u8(bytes)?;
            let fr = read_u8(bytes)?;
            let ff = read_u8(bytes)?;
            MetaEvent::SMTPEOffset { hh, mm, ss, fr, ff }
        }
        0x58 => {
            expect_u8(bytes, 4)?;
            let nn = read_u8(bytes)?;
            let dd = read_u8(bytes)?;
            let cc = read_u8(bytes)?;
            let bb = read_u8(bytes)?;
            MetaEvent::TimeSignature { nn, dd, cc, bb }
        }
        0x59 => {
            expect_u8(bytes, 2)?;
            let sf = read_u8(bytes)?;
            let mi = read_u8(bytes)?;
            MetaEvent::KeySignature { sf, mi }
        }
        0x7f => read_data(bytes).map(MetaEvent::SequencerSpecific)?,
        _ => {
            let data = read_data(bytes)?;
            MetaEvent::Unknown { meta_type, data }
        }
    };

    Ok(meta_event)
}

// https://www.midi.org/specifications/item/table-1-summary-of-midi-message
fn read_midi_event(bytes: &mut &[u8], status_byte: u8) -> Result<MidiEvent, ErrorKind> {
    let channel = status_byte & 0x0f;
    let status = status_byte & 0xf0;
    let kind = match status {
        0x80 => {
            let key = read_u7(bytes)?;
            let velocity = read_u7(bytes)?;
            MidiEventKind::NoteOff { key, velocity }
        }
        0x90 => {
            let key = read_u7(bytes)?;
            let velocity = read_u7(bytes)?;
            MidiEventKind::NoteOn { key, velocity }
        }
        0xa0 => {
            let key = read_u7(bytes)?;
            let velocity = read_u7(bytes)?;
            MidiEventKind::PolyphonicKeyPressure { key, velocity }
        }
        0xb0 => {
            let number = read_u7(bytes)?;
            match number {
                0x78 => expect_u8(bytes, 0).map(|_| MidiEventKind::AllSoundOff)?,
                0x79 => expect_u8(bytes, 0).map(|_| MidiEventKind::ResetAllControllers)?,
                0x7a => read_action(bytes).map(MidiEventKind::LocalControl)?,
                0x7b => expect_u8(bytes, 0).map(|_| MidiEventKind::AllNotesOff)?,
                0x7c => expect_u8(bytes, 0).map(|_| MidiEventKind::OmniModeOff)?,
                0x7d => expect_u8(bytes, 0).map(|_| MidiEventKind::OmniModeOn)?,
                0x7e => read_u8(bytes).map(MidiEventKind::MonoModeOn)?,
                0x7f => expect_u8(bytes, 0).map(|_| MidiEventKind::PolyModeOn)?,
                _ => {
                    let value = read_u7(bytes)?;
                    MidiEventKind::ControllerChange { number, value }
                }
            }
        }
        0xc0 => read_u7(bytes).map(MidiEventKind::ProgramChange)?,
        0xd0 => read_u7(bytes).map(MidiEventKind::ChannelKeyPressure)?,
        0xe0 => {
            let lsb = read_u7(bytes)?;
            let msb = read_u7(bytes)?;
            MidiEventKind::PitchBend { lsb, msb }
        }
        _ => {
            unimplemented!();
        }
    };

    let midi_event = MidiEvent { channel, kind };

    Ok(midi_event)
}

/// Low-level [`HeaderChunk`] reader.
///
/// Reads [`HeaderChunk`] and moves the cursor the beginning of the first
/// [`TrackChunk`]
///
/// # Example
///
/// ```
/// # use midi::{Error, read::read_header_chunk};
/// # fn foo(mut bytes: &[u8]) -> Result<(), Error> {
/// let cursor: &mut &[u8] = &mut bytes;
/// let header_chunk = read_header_chunk(cursor)?;
/// # Ok(())
/// # }
/// ```
///
/// [`HeaderChunk`]: struct.HeaderChunk.html
/// [`TrackChunk`]: struct.TrackChunk.html
pub fn read_header_chunk(cursor: &mut &[u8]) -> Result<HeaderChunk, Error> {
    // validate chunk type
    expect_bytes(cursor, b"MThd")
        .map_err(context("read_header_chunk: header type must be 'MThd'"))?;

    // validate header length
    expect_u32(cursor, 6).map_err(context("read_header_chunk: header data length should be 6"))?;

    // read header fields
    let format =
        read_format(cursor).map_err(context("read_header_chunk: header must specify format"))?;
    let tracks =
        read_u16(cursor).map_err(context("read_header_chunk: header must specify tracks"))?;
    let division =
        read_u16(cursor).map_err(context("read_header_chunk: header must specify division"))?;

    let header = HeaderChunk {
        format,
        tracks,
        division,
    };

    Ok(header)
}

/// Low-level [`TrackChunk`] reader.
///
/// Reads [`TrackChunk`] and moves the cursor the beginning of the next
/// [`TrackChunk`]
///
/// # Example
///
/// ```
/// # use midi::{
/// #   Error,
/// #   read::{read_track_chunk, read_header_chunk}
/// # };
/// # fn foo(mut bytes: &[u8]) -> Result<(), Error> {
/// let cursor: &mut &[u8] = &mut bytes;
/// let header_chunk = read_header_chunk(cursor)?;
/// for _ in 0..header_chunk.tracks {
///     let track_chunk = read_track_chunk(cursor)?;
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`TrackChunk`]: struct.TrackChunk.html
pub fn read_track_chunk<'a>(bytes: &mut &'a [u8]) -> Result<TrackChunk<'a>, Error> {
    // validate chunk type
    expect_bytes(bytes, b"MTrk").map_err(context("read_track_chunk: track type must be 'MTrk'"))?;

    // read track len
    let len = read_u32(bytes).map_err(context("read_track_chunk: track must specify len"))?;

    // read track data
    let data = read_bytes(bytes, len as usize)
        .map_err(context("read_track_chunk: track must contain event bytes"))?;

    let track_chunk = TrackChunk { data };

    Ok(track_chunk)
}

/// Low-level [`Event`] reader.
///
/// Reads [`Event`] and moves the cursor the beginning of the next
/// [`Event`]
///
/// # Example
///
/// ```
/// # use midi::{Error, read::read_event};
/// # fn foo(mut bytes: &[u8]) -> Result<(), Error> {
/// let cursor: &mut &[u8] = &mut bytes;
/// while !cursor.is_empty() {
///     let event = read_event(cursor)?;
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`Event`]: ../struct.Event.html
pub fn read_event<'a>(bytes: &mut &'a [u8]) -> Result<Event<'a>, Error> {
    // read time
    let time = read_vlq(bytes).map_err(context("read_event: event must have valid time"))?;

    // read event type
    let event_type = read_u8(bytes).map_err(context("read_event: event must have type"))?;

    // read event data
    let kind = match event_type {
        0xf0 => read_data(bytes)
            .map(SysexEvent::F0)
            .map(EventKind::Sysex)
            .map_err(context("read_event: failed to read sysex event"))?,
        0xf7 => read_data(bytes)
            .map(SysexEvent::F7)
            .map(EventKind::Sysex)
            .map_err(context("read_event: failed to read sysex event"))?,
        0xff => read_meta_event(bytes)
            .map(EventKind::Meta)
            .map_err(context("read_event: failed to read meta event"))?,
        _ => read_midi_event(bytes, event_type)
            .map(EventKind::Midi)
            .map_err(context("read_event: failed to read midi event"))?,
    };

    let event = Event { kind, time };

    Ok(event)
}

/// Specifies some basic information about the data in `SMF`.
#[derive(Debug, Clone, Copy)]
pub struct HeaderChunk {
    pub format: Format,
    pub tracks: u16,
    pub division: u16,
}

/// Lazy `SMF` reader.
pub struct SmfReader<'a> {
    header: HeaderChunk,
    // tracks chunks data
    data: &'a [u8],
}

impl<'a> SmfReader<'a> {
    /// Creates new [`SmfReader`].
    ///
    /// # Example
    ///
    /// ```
    /// # use midi::{Error, read::SmfReader};
    /// # fn foo(data: &[u8]) -> Result<(), Error> {
    /// let smf_reader = SmfReader::new(data)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`SmfReader`]: struct.SmfReader.html
    pub fn new(mut data: &'a [u8]) -> Result<Self, Error> {
        let cursor = &mut data;
        let header = read_header_chunk(cursor)?;
        let reader = Self {
            header,
            data: *cursor,
        };
        Ok(reader)
    }

    /// Reads [`HeaderChunk`].
    ///
    /// # Example
    ///
    /// ```
    /// # use midi::{Error, read::SmfReader};
    /// # fn foo(data: &[u8]) -> Result<(), Error> {
    /// # let smf_reader = SmfReader::new(data)?;
    /// let header_chunk = smf_reader.header_chunk();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`HeaderChunk`]: struct.HeaderChunk.html
    pub fn header_chunk(&self) -> HeaderChunk {
        self.header
    }

    /// Creates iterator over [`TrackChunk`]s.
    ///
    /// # Example
    ///
    /// ```
    /// # use midi::{Error, read::SmfReader};
    /// # fn foo(data: &[u8]) -> Result<(), Error> {
    /// # let smf_reader = SmfReader::new(data)?;
    /// let track_chunk = smf_reader.track_chunk_iter();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`TrackChunk`]: struct.TrackChunk.html
    pub fn track_chunk_iter(&self) -> impl Iterator<Item = Result<TrackChunk<'a>, Error>> {
        TrackChunkIter {
            data: self.data,
            tracks: self.header.tracks as usize,
        }
    }
}

struct TrackChunkIter<'a> {
    data: &'a [u8],
    tracks: usize,
}

impl<'a> Iterator for TrackChunkIter<'a> {
    type Item = Result<TrackChunk<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.tracks == 0 {
            if self.data.is_empty() {
                return None;
            }
            return Some(Err(Error {
                context: "TrackChunkIter::next: undread data left",
                kind: ErrorKind::Invalid,
            }));
        }

        self.tracks -= 1;
        let cursor = &mut self.data;
        let track_chunk = match read_track_chunk(cursor) {
            Ok(track_data) => track_data,
            Err(err) => return Some(Err(err)),
        };
        self.data = *cursor;

        Some(Ok(track_chunk))
    }
}

/// Iterator over [`Event`]s.
///
/// Created using [`SmfReader::track_chunk_iter`] method.
///
/// [`Event`]: ../struct.Event.html
/// [`SmfReader::track_chunk_iter`]:
/// struct.SmfReader.html#method.track_chunk_iter
pub struct TrackChunk<'a> {
    data: &'a [u8],
}

impl<'a> Iterator for TrackChunk<'a> {
    type Item = Result<Event<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }

        let cursor = &mut self.data;
        let event = match read_event(cursor) {
            Ok(event) => event,
            Err(err) => return Some(Err(err)),
        };
        self.data = *cursor;
        Some(Ok(event))
    }
}

#[cfg(test)]
mod tests {
    use super::{read_header_chunk, read_u16, read_u24, read_u32, read_u7, read_vlq};
    use crate::{ErrorKind, Format};
    use core::ops;

    fn test_cursor<'a, 'c>(data: &'c mut &'a [u8]) -> TestCursor<'a, 'c> {
        TestCursor(data)
    }

    /// Cursor which needs to be empty on drop
    struct TestCursor<'a, 'cursor>(&'cursor mut &'a [u8]);

    impl<'a, 'cursor> ops::Deref for TestCursor<'a, 'cursor> {
        type Target = &'a [u8];

        fn deref(&self) -> &Self::Target {
            self.0
        }
    }

    impl<'a, 'cursor> ops::DerefMut for TestCursor<'a, 'cursor> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.0
        }
    }

    impl<'a, 'cursor> Drop for TestCursor<'a, 'cursor> {
        fn drop(&mut self) {
            // compare it empty slice to print the content of the cursor
            // in case of the error
            assert_eq!(self.0, &&[]);
        }
    }

    #[test]
    fn test_read_vlq() {
        fn read_vlq_u(mut bytes: &[u8]) -> u32 {
            read_vlq(&mut test_cursor(&mut bytes)).unwrap()
        }
        assert_eq!(read_vlq_u(&[0]), 0);
        assert_eq!(read_vlq_u(&[0x7f]), 0x7f);
        assert_eq!(read_vlq_u(&[0x81, 0x00]), 0x80);
        assert_eq!(read_vlq_u(&[0xff, 0x7f]), 0x3fff);
        assert_eq!(read_vlq_u(&[0x87, 0x68]), 0x3e8);
        assert_eq!(read_vlq_u(&[0xbd, 0x84, 0x40]), 0xf4240);
    }

    #[test]
    fn test_read_u7() {
        fn read_u7_u(mut bytes: &[u8]) -> Result<u8, ErrorKind> {
            read_u7(&mut test_cursor(&mut bytes))
        }

        assert_eq!(read_u7_u(&[0]).unwrap(), 0);
        assert_eq!(read_u7_u(&[0x7f]).unwrap(), 0x7f);
        assert_eq!(read_u7_u(&[0x80]).unwrap_err(), ErrorKind::Invalid);
        assert_eq!(read_u7_u(&[0xff]).unwrap_err(), ErrorKind::Invalid);
    }

    #[test]
    fn test_read_u16() {
        fn read_u16_u(mut bytes: &[u8]) -> u16 {
            read_u16(&mut test_cursor(&mut bytes)).unwrap()
        }

        assert_eq!(read_u16_u(&[0, 6]), 6);
    }

    #[test]
    fn test_read_u24() {
        fn read_u24_u(mut bytes: &[u8]) -> u32 {
            read_u24(&mut test_cursor(&mut bytes)).unwrap()
        }

        assert_eq!(read_u24_u(&[0, 0, 6]), 6);
    }

    #[test]
    fn test_read_u32() {
        fn read_u32_u(mut bytes: &[u8]) -> u32 {
            read_u32(&mut test_cursor(&mut bytes)).unwrap()
        }

        assert_eq!(read_u32_u(&[0, 0, 0, 6]), 6);
    }

    #[test]
    fn test_read_header_chunk() {
        let mut data = &[77u8, 84, 104, 100, 0, 0, 0, 6, 0, 1, 0, 3, 4, 0] as &[u8];
        let header_chunk = read_header_chunk(&mut test_cursor(&mut data)).unwrap();
        assert_eq!(header_chunk.format, Format::MultiTrack);
        assert_eq!(header_chunk.tracks, 3);
        assert_eq!(header_chunk.division, 1024);
    }
}
