use midi;
use futures::io::AsyncRead;
use futures::stream::StreamExt;

async fn example_read<TRead: AsyncRead + Unpin>(mut io: TRead) -> Result<(), midi::Error> {
    let header = midi::read_header(&mut io).await?;
    for _ in 0 .. header.tracks {
        let mut chunk = midi::read_chunk(&mut io).await?;
        let mut events = chunk.events();
        for event in events.next().await {
            let event = event?;
        }
        
        //for event in chunk.events().next().await {
        //}
        //while let Some(event) = midi::read_event(&mut io).await? {
        //}
    }
    
    Ok(())
}

fn main() {
}
