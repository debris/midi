//! Standard Midi File (SMF) parser.
//!
//! If you want to parse the entire SMF at once, take a look at [`Smf`] struct.
//!
//! # Example
//!
//! Lazy reading using [`SmfReader`] without heap allocations
//!
//! ```
//! # use midi;
//! # fn no_allocation_read(bytes: &[u8]) -> Result<(), midi::Error> {
//! let smf = midi::read::SmfReader::new(bytes)?;
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

#![cfg_attr(not(feature = "alloc"), no_std)]

mod features;
pub mod read;

use core::str;
pub use features::*;

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
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ErrorKind {
    /// Non-recoverable.
    Fatal,
    /// Read data differs from expected data.
    Invalid,
}

/// `SMF` format specified in `MThd` chunk
#[derive(Debug, Clone, Copy, PartialEq)]
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
    NoteOff { key: u8, velocity: u8 },
    NoteOn { key: u8, velocity: u8 },
    PolyphonicKeyPressure { key: u8, velocity: u8 },
    ControllerChange { number: u8, value: u8 },
    ProgramChange(u8),
    ChannelKeyPressure(u8),
    PitchBend { lsb: u8, msb: u8 },

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
    Text(Text<'a>),
    CopyrightNotice(Text<'a>),
    Name(Text<'a>),
    InstrumentName(Text<'a>),
    Lyric(Text<'a>),
    Marker(Text<'a>),
    CuePoint(Text<'a>),
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
    },
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

/// [`MetaEvent`] text
///
/// [`MetaEvent`]: enum.MetaEvent.html
#[derive(Debug)]
pub struct Text<'a> {
    data: &'a [u8],
}

impl<'a> Text<'a> {
    /// Creates new [`Text`].
    ///
    /// [`Text`]: struct.Text.html
    pub fn new(data: &'a [u8]) -> Self {
        Text { data }
    }

    /// Try to decode text as utf8.
    pub fn as_utf8(&self) -> Result<&'a str, str::Utf8Error> {
        str::from_utf8(self.data)
    }

    /// Returns text slice.
    pub fn raw(&self) -> &'a [u8] {
        self.data
    }
}
