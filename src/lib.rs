//! MIDI streaming library.
//!
//! ```
//! # use midi;
//! # pub async fn read(io: &[u8]) -> Result<(), midi::Error> {
//! let header = midi::read_header(io).await?;
//! for _ in 0 .. header.tracks {
//!     let mut chunk = midi::read_chunk(io).await?;
//!     while let Some(_event) = midi::read_event(&mut chunk).await? {
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

#[derive(Debug, Default)]
pub struct MetaEvent;

#[derive(Debug)]
pub struct SysexEvent;

#[derive(Debug)]
pub enum Event {
    Midi(MidiEvent),
    Meta(MetaEvent),
    Sysex(SysexEvent),
}

#[derive(Debug)]
pub struct Chunk<T> {
    io: T,
}
