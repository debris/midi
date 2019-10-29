use std::convert::TryInto;
use std::str;
use crate::{
    SysexEvent, Error, ErrorKind, Format, Event, EventKind, 
    MetaEvent, Action, MidiEvent, MidiEventKind,
};

fn context(context: &'static str) -> impl FnOnce(ErrorKind) -> Error {
    move |kind| {
        Error {
            context,
            kind,
        }
    }
}

fn read_bytes<'a>(data: &mut &'a [u8], len: usize) -> Result<&'a [u8], ErrorKind> {
    if data.len() < len {
        return Err(ErrorKind::Fatal)
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
    read_bytes(data, 1)
        .map(|b| b[0])
        .map(u8::from_le)
}

fn read_u16(data: &mut &[u8]) -> Result<u16, ErrorKind> {
    read_bytes(data, 2)
        .map(|b| b.try_into().unwrap())
        .map(u16::from_le_bytes)
}

fn read_u24(data: &mut &[u8]) -> Result<u32, ErrorKind> {
    read_bytes(data, 3)
        .map(|b| {
            let mut bytes = [0u8; 4];
            bytes[..3].copy_from_slice(&b);
            bytes
        })
        .map(|b| b.try_into().unwrap())
        .map(u32::from_le_bytes)
}

fn read_u32(data: &mut &[u8]) -> Result<u32, ErrorKind> {
    read_bytes(data, 4)
        .map(|b| b.try_into().unwrap())
        .map(u32::from_le_bytes)
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
        return Err(ErrorKind::Invalid)
    }
    Ok(())
}

fn expect_u8(data: &mut &[u8], expected: u8) -> Result<(), ErrorKind> {
    if read_u8(data)? != expected {
        return Err(ErrorKind::Invalid)
    }
    Ok(())
}

fn expect_u32(data: &mut &[u8], expected: u32) -> Result<(), ErrorKind> {
    if read_u32(data)? != expected {
        return Err(ErrorKind::Invalid)
    }
    Ok(())
}

fn read_vlq(data: &mut &[u8]) -> Result<u32, ErrorKind> {
    let mut result: u32 = 0;
    let mut size: usize = 0;
    while {
        // vlq must fit into 32 bit integer
        if size > 3 {
            return Err(ErrorKind::Invalid)
        }

        let byte = read_u8(data)?;
        size += 1;
        result |= (byte & 0b0111_1111) as u32;
        (byte & 0b1000_0000) != 0
    } {
        result <<= 7;
    }

    Ok(u32::from_le(result))
}

fn read_data<'a>(data: &mut &'a [u8]) -> Result<&'a [u8], ErrorKind> {
    let length = read_vlq(data)?;
    read_bytes(data, length as usize)
}

fn read_text<'a>(data: &mut &'a [u8]) -> Result<&'a str, ErrorKind> {
    let text_data = read_data(data)?;
    str::from_utf8(text_data).map_err(|_| ErrorKind::Invalid)
}

fn read_action(data: &mut &[u8])-> Result<Action, ErrorKind> {
    let byte = read_u8(data)?;
    let action = match byte {
        0x00 => Action::Disconnect,
        0x7f => Action::Reconnect,
        _ => return Err(ErrorKind::Invalid),
    };

    Ok(action)
}

fn read_meta_event<'a>(bytes: &mut &'a [u8])-> Result<MetaEvent<'a>, ErrorKind> {
    let meta_type = read_u8(bytes)?;
    let meta_event = match meta_type {
        0x00 => {
            expect_u8(bytes, 2)?;
            let number = read_u16(bytes)?;
            MetaEvent::SequenceNumber(number) 
        },
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
        },
        0x2f => {
            expect_u8(bytes, 0)?;
            MetaEvent::EndOfTrack
        },
        0x51 => {
            expect_u8(bytes, 3)?;
            let tempo = read_u24(bytes)?;
            MetaEvent::SetTempo(tempo)
        },
        0x54 => {
            expect_u8(bytes, 5)?;
            let hh = read_u8(bytes)?;
            let mm = read_u8(bytes)?;
            let ss = read_u8(bytes)?;
            let fr = read_u8(bytes)?;
            let ff = read_u8(bytes)?;
            MetaEvent::SMTPEOffset {
                hh, mm, ss, fr, ff
            }
        },
        0x58 => {
            expect_u8(bytes, 4)?;
            let nn = read_u8(bytes)?;
            let dd = read_u8(bytes)?;
            let cc = read_u8(bytes)?;
            let bb = read_u8(bytes)?;
            MetaEvent::TimeSignature {
                nn, dd, cc, bb
            }
        },
        0x59 => {
            expect_u8(bytes, 2)?;
            let sf = read_u8(bytes)?;
            let mi = read_u8(bytes)?;
            MetaEvent::KeySignature {
                sf, mi
            }
        },
        0x7f => read_data(bytes).map(MetaEvent::SequencerSpecific)?,
        _ => {
            let data = read_data(bytes)?;
            MetaEvent::Unknown {
                meta_type,
                data,
            }
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
            MidiEventKind::NoteOff {
                key, velocity
            }
        },
        0x90 => {
            let key = read_u7(bytes)?;
            let velocity = read_u7(bytes)?;
            MidiEventKind::NoteOn {
                key, velocity
            }
        },
        0xa0 => {
            let key = read_u7(bytes)?;
            let velocity = read_u7(bytes)?;
            MidiEventKind::PolyphonicKeyPressure {
                key, velocity
            }
        },
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
                    MidiEventKind::ControllerChange {
                        number, value
                    }
                }
            }
        },
        0xc0 => read_u7(bytes).map(MidiEventKind::ProgramChange)?,
        0xd0 => read_u7(bytes).map(MidiEventKind::ChannelKeyPressure)?,
        0xe0 => {
            let lsb = read_u7(bytes)?;
            let msb = read_u7(bytes)?;
            MidiEventKind::PitchBend {
                lsb, msb
            }
        },
        _ => {
            unimplemented!();
        }
    };

    let midi_event = MidiEvent {
        channel,
        kind,
    };


    Ok(midi_event)
}

pub fn read_header(bytes: &mut &[u8]) -> Result<Header, Error> {
    // validate chunk type
    expect_bytes(bytes, b"MThd")
        .map_err(context("read_header: header type must be 'MThd'"))?;

    // validate header length
    expect_u32(bytes, 6)
        .map_err(context("read_header: header data length should be 6"))?;

    // read header fields
    let format = read_format(bytes)
        .map_err(context("read_header: header must specify format"))?;
    let tracks = read_u16(bytes)
        .map_err(context("read_header: header must specify tracks"))?;
    let division = read_u16(bytes)
        .map_err(context("read_header: header must specify division"))?;

    let header = Header {
        format,
        tracks,
        division,
    };

    Ok(header)
}

pub fn read_track<'a>(bytes: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    // validate chunk type
    expect_bytes(bytes, b"MTrk")
        .map_err(context("read_track: track type must be 'MTrk'"))?;

    // read track len
    let len = read_u32(bytes)
        .map_err(context("read_track: track must specify len"))?;

    // read track data
    let track_data = read_bytes(bytes, len as usize)
        .map_err(context("read_track: track must contain event bytes"))?;

    Ok(track_data)
}

pub fn read_event<'a>(bytes: &mut &'a [u8]) -> Result<Event<'a>, Error> {
    // read time
    let time = read_vlq(bytes)
        .map_err(context("read_event: event must have valid time"))?;

    // read event type
    let event_type = read_u8(bytes)
        .map_err(context("read_event: event must have type"))?;

    // read event data
    let kind = match event_type {
        0xf0 => read_data(bytes).map(SysexEvent::F0).map(EventKind::Sysex)
            .map_err(context("read_event: failed to read sysex event"))?,
        0xf7 => read_data(bytes).map(SysexEvent::F7).map(EventKind::Sysex)
            .map_err(context("read_event: failed to read sysex event"))?,
        0xff => read_meta_event(bytes).map(EventKind::Meta)
            .map_err(context("read_event: failed to read meta event"))?,
        _ => read_midi_event(bytes, event_type).map(EventKind::Midi)
            .map_err(context("read_event: failed to read midi event"))?,
    };
    
    let event = Event {
        kind,
        time,
    };

    Ok(event)
}

#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub format: Format,
    pub tracks: u16,
    pub division: u16,
}


pub struct SmfReader<'a> {
    header: Header,
    // tracks chunks data
    data: &'a [u8],
}

impl<'a> SmfReader<'a> {
    pub fn new(mut data: &'a [u8]) -> Result<Self, Error> {
        let cursor = &mut data;
        let header = read_header(cursor)?;
        let reader = Self {
            header,
            data: *cursor,
        };
        Ok(reader)
    }

    pub fn header(&self) -> Header {
        self.header
    }

    pub fn tracks(&self) -> TrackChunks<'a> {
        TrackChunks {
            data: self.data,
            tracks: self.header.tracks as usize,
        }
    }
}

pub struct TrackChunks<'a> {
    data: &'a [u8],
    tracks: usize,
}

impl<'a> Iterator for TrackChunks<'a> {
    type Item = Result<TrackChunkData<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.tracks == 0 {
            return None
        }

        self.tracks -= 1;
        let cursor = &mut self.data;
        let track_data = match read_track(cursor) {
            Ok(track_data) => track_data,
            Err(err) => return Some(Err(err)),
        };
        self.data = *cursor;

        let track_chunk_data = TrackChunkData {
            data: track_data,
        };

        Some(Ok(track_chunk_data))
    }
}

pub struct TrackChunkData<'a> {
    data: &'a [u8],
}

impl<'a>Iterator for TrackChunkData<'a> {
    type Item = Result<Event<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None
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
    use super::read_vlq;

    #[test]
    fn test_read_vlq() {
        fn read_vlq_u(mut bytes: &[u8]) -> u32 {
            read_vlq(&mut bytes).unwrap()
        }
        assert_eq!(read_vlq_u(&[0]), 0);
        assert_eq!(read_vlq_u(&[0x7f]), 0x7f);
        assert_eq!(read_vlq_u(&[0x81, 0x00]), 0x80);
        assert_eq!(read_vlq_u(&[0xff, 0x7f]), 0x3fff);
        assert_eq!(read_vlq_u(&[0x87, 0x68]), 0x3e8);
        assert_eq!(read_vlq_u(&[0xbd, 0x84, 0x40]), 0xf4240);
    }
}
