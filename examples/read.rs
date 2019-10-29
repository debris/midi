use midi;

fn no_allocation_read(bytes: &[u8]) -> Result<(), midi::Error> {
    let smf = midi::read::SmfReader::new(bytes)?;
    let _header = smf.header_chunk();
    let track_chunks = smf.track_chunk_iter();
    for track_chunk in track_chunks {
        let events = track_chunk?;
        for event in events {
            let _event = event?;
        }
    }
    
    Ok(())
}

fn main() {
}
