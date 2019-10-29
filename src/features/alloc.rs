/// Crate options behind `alloc` feature.
extern crate alloc;

use crate::{read, Error, Event, Format};
use alloc::vec::Vec;

/// `MTrk` chunk.
#[derive(Debug)]
pub struct Track<'a> {
    pub events: Vec<Event<'a>>,
}

/// Standard Midi File.
///
/// # Example
///
/// ```
/// # use midi;
/// # fn just_read(bytes: &[u8]) -> Result<(), midi::Error> {
/// let smf = midi::Smf::read(bytes)?;
/// let format = smf.format;
/// let division = smf.division;
/// for track in smf.tracks {
///     for event in track.events {
///     }
/// }
/// # Ok(())
/// # }
///
/// ```
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
