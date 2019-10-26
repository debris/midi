use std::convert::TryInto;
use futures::io::{self, AsyncRead, AsyncReadExt, Take};
use crate::{Header, Error, Format, Chunk, Event, SysexEvent, MidiEvent, MetaEvent};

async fn starts_with<TRead: AsyncRead + Unpin>(mut io: TRead, bytes: &[u8; 4]) -> bool {
    let mut data = [0u8; 4];
    // error here can be ignored, cause if reading fails, bytes != data
    let _ = io.read_exact(&mut data).await;
    bytes == &data
}

async fn read_byte<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u8, io::Error> {
    let mut data = [0u8; 1];
    let _ = io.read_exact(&mut data).await?;
    Ok(data[0])
}

async fn assert_byte<TRead: AsyncRead + Unpin>(io: TRead, byte: u8) -> Result<(), io::Error> {
    let b = read_byte(io).await?;
    if b == byte {
        Ok(())
    } else {
        // TODO: be more descriptive
        Err(io::ErrorKind::InvalidData.into())
    }
}

async fn read_u16<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u16, io::Error> {
    let mut data = [0u8; 2];
    let _ = io.read_exact(&mut data).await?;
    Ok(u16::from_le_bytes(data.try_into().unwrap()))
}

async fn read_u24<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u32, io::Error> {
    let mut data = [0u8; 4];
    let _ = io.read_exact(&mut data[0..3]).await?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}

async fn read_u32<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u32, io::Error> {
    let mut data = [0u8; 4];
    let _ = io.read_exact(&mut data).await?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}

async fn read_vlq<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u32, io::Error> {
    let mut result: u32 = 0;
    let mut size: usize = 0;
    while {
        // vlq must fit into 32 bit integer
        if size > 3 {
            return Err(io::ErrorKind::InvalidData.into())
        }

        let byte = read_byte(&mut io).await?;
        size += 1;
        result |= (byte & 0b0111_1111) as u32;
        (byte & 0b1000_0000) != 0
    } {
        result <<= 7;
    }

    Ok(u32::from_le(result))
}

async fn read_data<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<Vec<u8>, io::Error> {
    let length = read_vlq(&mut io).await?;
    let mut data = vec![0u8; length as usize];
    io.read_exact(&mut data).await?;
    Ok(data)
}

async fn read_text<TRead: AsyncRead + Unpin>(io: TRead) -> Result<String, io::Error> {
    read_data(io).await.map(|data| String::from_utf8(data).expect("TODO"))
}

async fn read_meta_event<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<MetaEvent, io::Error> {
    let meta_type = read_byte(&mut io).await?;
    let meta_event = match meta_type {
        0x00 => {
            assert_byte(&mut io, 2).await?;
            let number = read_u16(&mut io).await?;
            MetaEvent::SequenceNumber(number) 
        },
        0x01 => read_text(&mut io).await.map(MetaEvent::Text)?,
        0x02 => read_text(&mut io).await.map(MetaEvent::CopyrightNotice)?,
        0x03 => read_text(&mut io).await.map(MetaEvent::Name)?,
        0x04 => read_text(&mut io).await.map(MetaEvent::InstrumentName)?,
        0x05 => read_text(&mut io).await.map(MetaEvent::Lyric)?,
        0x06 => read_text(&mut io).await.map(MetaEvent::Marker)?,
        0x07 => read_text(&mut io).await.map(MetaEvent::CuePoint)?,
        0x20 => {
            assert_byte(&mut io, 1).await?;
            let channel = read_byte(&mut io).await?;
            MetaEvent::ChannelPrefix(channel)
        },
        0x2f => {
            assert_byte(&mut io, 0).await?;
            MetaEvent::EndOfTrack
        },
        0x51 => {
            assert_byte(&mut io, 3).await?;
            let tempo = read_u24(&mut io).await?;
            MetaEvent::SetTempo(tempo)
        },
        0x54 => {
            assert_byte(&mut io, 5).await?;
            let hh = read_byte(&mut io).await?;
            let mm = read_byte(&mut io).await?;
            let ss = read_byte(&mut io).await?;
            let fr = read_byte(&mut io).await?;
            let ff = read_byte(&mut io).await?;
            MetaEvent::SMTPEOffset {
                hh, mm, ss, fr, ff
            }
        },
        0x58 => {
            assert_byte(&mut io, 4).await?;
            let nn = read_byte(&mut io).await?;
            let dd = read_byte(&mut io).await?;
            let cc = read_byte(&mut io).await?;
            let bb = read_byte(&mut io).await?;
            MetaEvent::TimeSignature {
                nn, dd, cc, bb
            }
        },
        0x59 => {
            assert_byte(&mut io, 2).await?;
            let sf = read_byte(&mut io).await?;
            let mi = read_byte(&mut io).await?;
            MetaEvent::KeySignature {
                sf, mi
            }
        },
        0x7f => read_data(&mut io).await.map(MetaEvent::SequencerSpecific)?,
        _ => {
            let data = read_data(&mut io).await?;
            MetaEvent::Unknown {
                meta_type,
                data,
            }
        }
    };

    Ok(meta_event)
}

pub async fn read_header<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<Header, Error> {
    // validate chunk type
    if !starts_with(&mut io, b"MThd").await {
        return Err(Error::HeaderLength)
    }

    // validate header length
    let _ = read_u32(&mut io).await.ok()
        .filter(|length| *length == 6)
        .ok_or_else(|| Error::HeaderLength)?;

    // read format
    let format = read_u16(&mut io).await.ok()
        .and_then(|format| match format {
            0 => Some(Format::Single),
            1 => Some(Format::MultiTrack),
            2 => Some(Format::MultiSequence),
            _ => None,
        })
        .ok_or_else(|| Error::HeaderFormat)?;

    // read tracks 
    let tracks = read_u16(&mut io).await.map_err(|_| Error::HeaderTracks)?;
    let division = read_u16(&mut io).await.map_err(|_| Error::HeaderDivision)?;

    let header = Header {
        format,
        tracks,
        division,
    };

    Ok(header)
}

pub async fn read_chunk<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<Chunk<Take<TRead>>, Error> {
    // validate chunk type
    if !starts_with(&mut io, b"MTrk").await {
        return Err(Error::TrackType)
    }

    // read chunk length
    let length = read_u32(&mut io).await.map_err(|_| Error::TrackLength)?;

    let chunk = Chunk {
        io: io.take(length as u64),
    };

    Ok(chunk)
}

pub async fn read_event<TRead: AsyncRead + Unpin>(chunk: &mut Chunk<TRead>) -> Result<Option<(u32, Event)>, Error> {
    // read time since previous event
    let time = match read_vlq(&mut chunk.io).await {
        Ok(time) => time,
        Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
            return Ok(None)
        },
        Err(_) => return Err(Error::EventData),
    };

    // read event type
    let event_type = read_byte(&mut chunk.io).await.map_err(|_| Error::EventData)?;

    let event = match event_type {
        0xf0 => read_data(&mut chunk.io).await.map(SysexEvent::F0).map(Event::Sysex).map_err(|_| Error::EventData)?,
        0xf7 => read_data(&mut chunk.io).await.map(SysexEvent::F7).map(Event::Sysex).map_err(|_| Error::EventData)?,
        0xff => {
            let meta_event = read_meta_event(&mut chunk.io).await.map_err(|_| Error::EventData)?;
            Event::Meta(meta_event)
        },
        _ => {
            Event::Midi(MidiEvent)
        }
    };

    // TODO: read the rest of the event
    Ok(Some((time, event)))
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use super::read_vlq;

    #[test]
    fn test_read_vlq() {
        fn read_vlq_sync(bytes: &[u8]) -> u32 {
            read_vlq(bytes).now_or_never().unwrap().unwrap()
        }
        assert_eq!(read_vlq_sync(&[0]), 0);
        assert_eq!(read_vlq_sync(&[0x7f]), 0x7f);
        assert_eq!(read_vlq_sync(&[0x81, 0x00]), 0x80);
        assert_eq!(read_vlq_sync(&[0xff, 0x7f]), 0x3fff);
        assert_eq!(read_vlq_sync(&[0x87, 0x68]), 0x3e8);
        assert_eq!(read_vlq_sync(&[0xbd, 0x84, 0x40]), 0xf4240);
    }
}
