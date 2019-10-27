//! MIDI streaming library.
//!
//! ```
//! # use midi;
//! # pub async fn read(io: &[u8]) -> Result<(), midi::Error> {
//! let header = midi::read_header(io).await?;
//! for _ in 0 .. header.tracks {
//!     let mut chunk = midi::read_chunk(io).await?;
//!     while let Some((time, event)) = midi::read_event(&mut chunk).await? {
//!            
//!     }
//! }
//! #   
//! #   Ok(())
//! # }
//! 
//! ```
//!
//! [Documentation] 
//!
//! [Documentation]: http://www.ccarh.org/courses/253/handout/smf/

mod read;

use std::borrow::Cow;
pub use read::{read_header, read_chunk, read_event};

/// MIDI header chunk
pub struct Header {
    pub format: Format,
    pub tracks: u16,
    pub division: u16,
}

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
    EventData,
}

/// MIDI file format
#[derive(Debug)]
pub enum Format {
    Single,
    MultiTrack,
    MultiSequence,
}

#[derive(Debug)]
pub enum MetaType {
    SequenceNumber,
    TextEvent,
}

#[derive(Debug)]
pub struct MidiEvent;

#[derive(Debug)]
pub enum MetaEvent<'a> {
    SequenceNumber(u16),
    Text(Cow<'a, str>),
    CopyrightNotice(Cow<'a, str>),
    Name(Cow<'a, str>),
    InstrumentName(Cow<'a, str>),
    Lyric(Cow<'a, str>),
    Marker(Cow<'a, str>),
    CuePoint(Cow<'a, str>),
    ChannelPrefix(u8),
    EndOfTrack,
    SetTempo(u32),
    SMTPEOffset {
        hh: u8,
        mm: u8,
        ss: u8,
        fr: u8,
        ff: u8,
    },
    TimeSignature {
        nn: u8,
        dd: u8,
        cc: u8,
        bb: u8,
    },
    KeySignature {
        sf: u8,
        mi: u8,
    },
    SequencerSpecific(Cow<'a, [u8]>),
    Unknown {
        meta_type: u8,
        data: Cow<'a, [u8]>,
    }
}

#[derive(Debug)]
pub enum SysexEvent<'a> {
    F0(Cow<'a, [u8]>),
    F7(Cow<'a, [u8]>),
}

#[derive(Debug)]
pub enum Event<'a> {
    Midi(MidiEvent),
    Meta(MetaEvent<'a>),
    Sysex(SysexEvent<'a>),
}

#[derive(Debug)]
pub struct Chunk<T> {
    io: T,
}
