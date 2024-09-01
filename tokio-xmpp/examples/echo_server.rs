use futures::{SinkExt, StreamExt};
use tokio::{self, io, net::TcpSocket};

use tokio_xmpp::parsers::stream_features::StreamFeatures;
use tokio_xmpp::xmlstream::{accept_stream, StreamHeader, Timeouts};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // TCP socket
    let address = "127.0.0.1:5222".parse().unwrap();
    let socket = TcpSocket::new_v4()?;
    socket.bind(address)?;

    let listener = socket.listen(1024)?;

    // Main loop, accepts incoming connections
    loop {
        let (stream, _addr) = listener.accept().await?;
        let stream = accept_stream(
            tokio::io::BufStream::new(stream),
            tokio_xmpp::parsers::ns::DEFAULT_NS,
            Timeouts::default(),
        )
        .await?;
        let stream = stream.send_header(StreamHeader::default()).await?;
        let mut stream = stream
            .send_features::<minidom::Element>(&StreamFeatures::default())
            .await?;

        tokio::spawn(async move {
            while let Some(packet) = stream.next().await {
                match packet {
                    Ok(packet) => {
                        println!("Received packet: {:?}", packet);
                        stream.send(&packet).await.unwrap();
                    }
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                    }
                }
            }
        });
    }
}
