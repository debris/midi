use midi;
use futures::io::AsyncRead;

async fn example_read<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<(), midi::Error> {
    let header = midi::read_header(&mut io).await?;
    for _ in 0 .. header.tracks {
        let mut chunk = midi::read_chunk(&mut io).await?;
        while let Some(_event) = midi::read_event(&mut chunk).await? {
            
        }
    }
    
    Ok(())
}

fn main() {

}
