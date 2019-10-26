use std::convert::TryInto;
use futures::io::{self, AsyncRead, AsyncReadExt, Take};
use crate::{Header, Error, Format, Chunk, Event, SysexEvent, MidiEvent, MetaEvent};

pub async fn starts_with<TRead: AsyncRead + Unpin>(mut io: TRead, bytes: &[u8; 4]) -> bool {
    let mut data = [0u8; 4];
    // error here can be ignored, cause if reading fails, bytes != data
    let _ = io.read_exact(&mut data).await;
    bytes == &data
}

pub async fn read_byte<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u8, io::Error> {
    let mut data = [0u8; 1];
    let _ = io.read_exact(&mut data).await?;
    Ok(data[0])
}

pub async fn read_u16<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u16, io::Error> {
    let mut data = [0u8; 2];
    let _ = io.read_exact(&mut data).await?;
    Ok(u16::from_le_bytes(data.try_into().unwrap()))
}

pub async fn read_u32<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u32, io::Error> {
    let mut data = [0u8; 4];
    let _ = io.read_exact(&mut data).await?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}

pub async fn read_vlq<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<u32, io::Error> {
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
        0x7f => {
            Event::Sysex(SysexEvent)
        },
        0xff => {
            Event::Meta(MetaEvent)
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
