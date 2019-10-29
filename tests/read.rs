use midi;

fn test_data(data: &[u8]) {
    let smf_reader = midi::read::SmfReader::new(data).unwrap();
    let track_chunks = smf_reader.track_chunk_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let _tracks: Vec<Vec<midi::Event>> = track_chunks
        .into_iter()
        .map(|events| events.collect::<Result<Vec<_>, _>>())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
}

#[test]
fn test_smf_reader() {
    test_data(include_bytes!("res/super_mario_64.mid"));
    test_data(include_bytes!("res/pirates.mid"));
}
