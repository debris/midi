//! Crate options behind `alloc` feature.

extern crate alloc;

use alloc::vec::Vec;
use crate::{Event, Format, Error, read};

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
    pub fn read_bytes(data: &'a [u8]) -> Result<Self, Error> {
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
