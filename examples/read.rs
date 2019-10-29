use midi;

fn no_allocation_read(bytes: &[u8]) -> Result<(), midi::Error> {
    let smf = midi::read::SmfReader::new(bytes)?;
    let _header = smf.header();
    let tracks = smf.tracks();
    for track in tracks {
        let track = track?;
        for event in track {
            let _event = event?;
        }
    }
    
    Ok(())
}

fn main() {
}
