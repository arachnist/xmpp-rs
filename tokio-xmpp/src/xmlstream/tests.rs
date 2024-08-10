// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use futures::{SinkExt, StreamExt};

use xmpp_parsers::stream_features::StreamFeatures;

use super::*;

#[derive(FromXml, AsXml, Debug)]
#[xml(namespace = "urn:example", name = "data")]
struct Data {
    #[xml(text)]
    contents: String,
}

#[tokio::test]
async fn test_initiate_accept_stream() {
    let (lhs, rhs) = tokio::io::duplex(65536);
    let initiator = tokio::spawn(async move {
        let mut stream = initiate_stream(
            tokio::io::BufStream::new(lhs),
            "jabber:client",
            StreamHeader {
                from: Some("client".into()),
                to: Some("server".into()),
                id: Some("client-id".into()),
            },
        )
        .await?;
        Ok::<_, io::Error>(stream.take_header())
    });
    let responder = tokio::spawn(async move {
        let stream = accept_stream(tokio::io::BufStream::new(rhs), "jabber:client").await?;
        assert_eq!(stream.header().from.unwrap(), "client");
        assert_eq!(stream.header().to.unwrap(), "server");
        assert_eq!(stream.header().id.unwrap(), "client-id");
        stream
            .send_header(StreamHeader {
                from: Some("server".into()),
                to: Some("client".into()),
                id: Some("server-id".into()),
            })
            .await
    });
    responder.await.unwrap().expect("responder");
    let server_header = initiator.await.unwrap().expect("initiator");
    assert_eq!(server_header.from.unwrap(), "server");
    assert_eq!(server_header.to.unwrap(), "client");
    assert_eq!(server_header.id.unwrap(), "server-id");
}

#[tokio::test]
async fn test_exchange_stream_features() {
    let (lhs, rhs) = tokio::io::duplex(65536);
    let initiator = tokio::spawn(async move {
        let stream = initiate_stream(
            tokio::io::BufStream::new(lhs),
            "jabber:client",
            StreamHeader::default(),
        )
        .await?;
        let (features, _) = stream.recv_features::<Data>().await?;
        Ok::<_, io::Error>(features)
    });
    let responder = tokio::spawn(async move {
        let stream = accept_stream(tokio::io::BufStream::new(rhs), "jabber:client").await?;
        let stream = stream.send_header(StreamHeader::default()).await?;
        stream
            .send_features::<Data>(&StreamFeatures::default())
            .await?;
        Ok::<_, io::Error>(())
    });
    responder.await.unwrap().expect("responder failed");
    let features = initiator.await.unwrap().expect("initiator failed");
    assert_eq!(features, StreamFeatures::default());
}

#[tokio::test]
async fn test_exchange_data() {
    let (lhs, rhs) = tokio::io::duplex(65536);

    let initiator = tokio::spawn(async move {
        let stream = initiate_stream(
            tokio::io::BufStream::new(lhs),
            "jabber:client",
            StreamHeader::default(),
        )
        .await?;
        let (_, mut stream) = stream.recv_features::<Data>().await?;
        stream
            .send(&Data {
                contents: "hello".to_owned(),
            })
            .await?;
        match stream.next().await {
            Some(Ok(Data { contents })) => assert_eq!(contents, "world!"),
            other => panic!("unexpected stream message: {:?}", other),
        }
        Ok::<_, io::Error>(())
    });

    let responder = tokio::spawn(async move {
        let stream = accept_stream(tokio::io::BufStream::new(rhs), "jabber:client").await?;
        let stream = stream.send_header(StreamHeader::default()).await?;
        let mut stream = stream
            .send_features::<Data>(&StreamFeatures::default())
            .await?;
        stream
            .send(&Data {
                contents: "world!".to_owned(),
            })
            .await?;
        match stream.next().await {
            Some(Ok(Data { contents })) => assert_eq!(contents, "hello"),
            other => panic!("unexpected stream message: {:?}", other),
        }
        Ok::<_, io::Error>(())
    });

    responder.await.unwrap().expect("responder failed");
    initiator.await.unwrap().expect("initiator failed");
}

#[tokio::test]
async fn test_clean_shutdown() {
    let (lhs, rhs) = tokio::io::duplex(65536);

    let initiator = tokio::spawn(async move {
        let stream = initiate_stream(
            tokio::io::BufStream::new(lhs),
            "jabber:client",
            StreamHeader::default(),
        )
        .await?;
        let (_, mut stream) = stream.recv_features::<Data>().await?;
        stream.close().await?;
        match stream.next().await {
            Some(Err(ReadError::StreamFooterReceived)) => (),
            other => panic!("unexpected stream message: {:?}", other),
        }
        Ok::<_, io::Error>(())
    });

    let responder = tokio::spawn(async move {
        let stream = accept_stream(tokio::io::BufStream::new(rhs), "jabber:client").await?;
        let stream = stream.send_header(StreamHeader::default()).await?;
        let mut stream = stream
            .send_features::<Data>(&StreamFeatures::default())
            .await?;
        match stream.next().await {
            Some(Err(ReadError::StreamFooterReceived)) => (),
            other => panic!("unexpected stream message: {:?}", other),
        }
        stream.close().await?;
        Ok::<_, io::Error>(())
    });

    responder.await.unwrap().expect("responder failed");
    initiator.await.unwrap().expect("initiator failed");
}

#[tokio::test]
async fn test_exchange_data_stream_reset_and_shutdown() {
    let (lhs, rhs) = tokio::io::duplex(65536);

    let initiator = tokio::spawn(async move {
        let stream = initiate_stream(
            tokio::io::BufStream::new(lhs),
            "jabber:client",
            StreamHeader::default(),
        )
        .await?;
        let (_, mut stream) = stream.recv_features::<Data>().await?;
        stream
            .send(&Data {
                contents: "hello".to_owned(),
            })
            .await?;
        match stream.next().await {
            Some(Ok(Data { contents })) => assert_eq!(contents, "world!"),
            other => panic!("unexpected stream message: {:?}", other),
        }
        let stream = stream
            .initiate_reset()
            .send_header(StreamHeader {
                from: Some("client".into()),
                to: Some("server".into()),
                id: Some("client-id".into()),
            })
            .await?;
        assert_eq!(stream.header().from.unwrap(), "server");
        assert_eq!(stream.header().to.unwrap(), "client");
        assert_eq!(stream.header().id.unwrap(), "server-id");

        let (_, mut stream) = stream.recv_features::<Data>().await?;
        stream
            .send(&Data {
                contents: "once more".to_owned(),
            })
            .await?;
        stream.close().await?;
        match stream.next().await {
            Some(Ok(Data { contents })) => assert_eq!(contents, "hello world!"),
            other => panic!("unexpected stream message: {:?}", other),
        }
        match stream.next().await {
            Some(Err(ReadError::StreamFooterReceived)) => (),
            other => panic!("unexpected stream message: {:?}", other),
        }
        Ok::<_, io::Error>(())
    });

    let responder = tokio::spawn(async move {
        let stream = accept_stream(tokio::io::BufStream::new(rhs), "jabber:client").await?;
        let stream = stream.send_header(StreamHeader::default()).await?;
        let mut stream = stream
            .send_features::<Data>(&StreamFeatures::default())
            .await?;
        stream
            .send(&Data {
                contents: "world!".to_owned(),
            })
            .await?;
        match stream.next().await {
            Some(Ok(Data { contents })) => assert_eq!(contents, "hello"),
            other => panic!("unexpected stream message: {:?}", other),
        }
        let stream = stream.accept_reset().await?;
        assert_eq!(stream.header().from.unwrap(), "client");
        assert_eq!(stream.header().to.unwrap(), "server");
        assert_eq!(stream.header().id.unwrap(), "client-id");
        let stream = stream
            .send_header(StreamHeader {
                from: Some("server".into()),
                to: Some("client".into()),
                id: Some("server-id".into()),
            })
            .await?;
        let mut stream = stream
            .send_features::<Data>(&StreamFeatures::default())
            .await?;
        stream
            .send(&Data {
                contents: "hello world!".to_owned(),
            })
            .await?;
        match stream.next().await {
            Some(Ok(Data { contents })) => assert_eq!(contents, "once more"),
            other => panic!("unexpected stream message: {:?}", other),
        }
        stream.close().await?;
        match stream.next().await {
            Some(Err(ReadError::StreamFooterReceived)) => (),
            other => panic!("unexpected stream message: {:?}", other),
        }
        Ok::<_, io::Error>(())
    });

    responder.await.unwrap().expect("responder failed");
    initiator.await.unwrap().expect("initiator failed");
}
