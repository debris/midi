//! MIDI streaming library.
//!
//! ```
//! # use midi;
//! # fn no_allocation_read(mut bytes: &[u8]) -> Result<(), midi::Error> {
//! let cursor: &mut &[u8] = &mut bytes;
//! let header = midi::read_header(cursor)?;
//! for _ in 0 .. header.tracks {
//! 	let mut track_data = midi::read_track_data(cursor)?;
//!  	let track_data_cursor = &mut track_data;
//! 	while !track_data_cursor.is_empty() {
//!  	let _event = midi::read_event(track_data_cursor)?;
//!
//! 	}
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [Documentation] 
//!
//! [Documentation]: http://www.ccarh.org/courses/253/handout/smf/

mod read;

pub use read::{read_smf, read_header, read_track, read_track_data, read_event};

/// MIDI header chunk
pub struct Header {
    pub format: Format,
    pub tracks: u16,
    pub division: u16,
}

#[derive(Debug)]
pub struct Error {
    context: &'static str,
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    Fatal,
    Invalid,
}

/// MIDI file format
#[derive(Debug, Clone, Copy)]
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
pub struct MidiEvent {
    pub channel: u8,
    pub kind: MidiEventKind,
}

#[derive(Debug)]
pub enum Action {
    Disconnect,
    Reconnect,
}

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

#[derive(Debug)]
pub enum SysexEvent<'a> {
    F0(&'a [u8]),
    F7(&'a [u8]),
}

#[derive(Debug)]
pub enum EventKind<'a> {
    Midi(MidiEvent),
    Meta(MetaEvent<'a>),
    Sysex(SysexEvent<'a>),
}

#[derive(Debug)]
pub struct Event<'a> {
    pub time: u32,
    pub kind: EventKind<'a>, 
}

pub struct Track<'a> {
    pub events: Vec<Event<'a>>,
}

pub struct Smf<'a> {
    pub format: Format,
    pub tracks: Vec<Track<'a>>,
    pub division: u16,
}

impl<'a> Smf<'a> {
    pub fn read(mut data: &'a [u8]) -> Result<Self, Error> {
        read_smf(&mut data)
    }
}
