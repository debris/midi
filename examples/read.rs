use midi;

fn no_allocation_read(mut bytes: &[u8]) -> Result<(), midi::Error> {
    let cursor = &mut bytes;
    let header = midi::read_header(cursor)?;
    for _ in 0 .. header.tracks {
        let mut track_data = midi::read_track_data(cursor)?;
        let track_data_cursor = &mut track_data;
        while !track_data_cursor.is_empty() {
            let _event = midi::read_event(track_data_cursor)?;

        }
    }
    
    Ok(())
}

fn main() {
}
