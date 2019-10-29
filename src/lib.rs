//! Standard Midi File (SMF) parser.
//!
//! # Examples
//!
//! `DOM` reading using [`Smf`]
//!
//! ```
//! # use midi;
//! # fn just_read(bytes: &[u8]) -> Result<(), midi::Error> {
//! let smf = midi::Smf::read(bytes)?;
//! let format = smf.format;
//! let division = smf.division;
//! for track in smf.tracks {
//!     for event in track.events {
//!     }
//! }
//! # Ok(())
//! # }
//!
//! ```
//!
//! Lazy reading using [`SmfReader`] without heap allocations
//!
//! ```
//! # use midi;
//! # fn no_allocation_read(bytes: &[u8]) -> Result<(), midi::Error> {
//! let smf = midi::read::SmfReader::new(bytes)?;
//! let header = smf.header_chunk();
//! let format = header.format;
//! let division = header.division;
//! let track_chunks = smf.track_chunk_iter();
//! for track_chunk in track_chunks {
//!     let events = track_chunk?;
//!     for event in events {
//!         let event = event?;
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//! [`Smf`]: struct.Smf.html
//! [`SmfReader`]: read/struct.SmfReader.html

pub mod read;

/// `SMF` reader error.
#[derive(Debug)]
pub struct Error {
    /// Error context description.
    pub context: &'static str,
    /// Type of error.
    pub kind: ErrorKind,
}

/// [`Error`] type.
///
/// [`Error`]: struct.Error.html
#[derive(Debug)]
pub enum ErrorKind {
    /// Non-recoverable.
    Fatal,
    /// Read data differs from expected data.
    Invalid,
}

/// `SMF` format specified in `MThd` chunk
#[derive(Debug, Clone, Copy)]
pub enum Format {
    Single,
    MultiTrack,
    MultiSequence,
}

/// [`Event`] variant.
///
/// [`Event`]: struct.Event.html
#[derive(Debug)]
pub struct MidiEvent {
    pub channel: u8,
    pub kind: MidiEventKind,
}

/// [`MidiEventKind::LocalControl`] action.
///
/// [`MidiEventKind::LocalControl`]: 
/// enum.MidiEventKind.html#variant.LocalControl
#[derive(Debug)]
pub enum Action {
    Disconnect,
    Reconnect,
}

/// [`MidiEvent`] variants.
///
/// [`MidiEvent`]: struct.MidiEvent.html
#[derive(Debug)]
pub enum MidiEventKind {
    NoteOff {
        key: u8,
        velocity: u8,
    },
    NoteOn {
        key: u8,
        velocity: u8,
    },
    PolyphonicKeyPressure {
        key: u8,
        velocity: u8,
    },
    ControllerChange {
        number: u8,
        value: u8,
    },
    ProgramChange(u8),
    ChannelKeyPressure(u8),
    PitchBend {
        lsb: u8,
        msb: u8,
    },

    AllSoundOff,
    ResetAllControllers,
    LocalControl(Action),
    AllNotesOff,
    OmniModeOff,
    OmniModeOn,
    MonoModeOn(u8),
    PolyModeOn,
}

/// [`Event`] variant.
///
/// [`Event`]: struct.Event.html
#[derive(Debug)]
pub enum MetaEvent<'a> {
    SequenceNumber(u16),
    Text(&'a str),
    CopyrightNotice(&'a str),
    Name(&'a str),
    InstrumentName(&'a str),
    Lyric(&'a str),
    Marker(&'a str),
    CuePoint(&'a str),
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
    SequencerSpecific(&'a [u8]),
    Unknown {
        meta_type: u8,
        data: &'a [u8],
    }
}

/// [`Event`] variant.
///
/// [`Event`]: struct.Event.html
#[derive(Debug)]
pub enum SysexEvent<'a> {
    F0(&'a [u8]),
    F7(&'a [u8]),
}

/// [`Event`] variants.
///
/// [`Event`]: struct.Event.html
#[derive(Debug)]
pub enum EventKind<'a> {
    Midi(MidiEvent),
    Meta(MetaEvent<'a>),
    Sysex(SysexEvent<'a>),
}

/// `MTrk` event.
#[derive(Debug)]
pub struct Event<'a> {
    pub time: u32,
    pub kind: EventKind<'a>, 
}


/// `MTrk` chunk.
#[derive(Debug)]
pub struct Track<'a> {
    pub events: Vec<Event<'a>>,
}

/// Standard Midi File.
#[derive(Debug)]
pub struct Smf<'a> {
    pub format: Format,
    pub tracks: Vec<Track<'a>>,
    pub division: u16,
}

impl<'a> Smf<'a> {
    pub fn read(data: &'a [u8]) -> Result<Self, Error> {
        let reader = read::SmfReader::new(data)?;
        let header = reader.header_chunk();
        let mut tracks = Vec::with_capacity(header.tracks as usize);
        let track_chunks = reader.track_chunk_iter();
        for track_chunk_data in track_chunks {
            let events = track_chunk_data?;
            let track = Track {
                events: events.collect::<Result<Vec<_>, _>>()?,
            };
            tracks.push(track);
        }

        let smf = Smf {
            format: header.format,
            tracks,
            division: header.division,
        };

        Ok(smf)
    }
}
