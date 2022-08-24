// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use s2n_quic::{Server};
use std::{error::Error};
use tokio::sync::mpsc;
use std::time::Duration;
use bytes::Bytes;

/// NOTE: this certificate is to be used for demonstration purposes only!
pub static CERT_PEM: &str = include_str!(
    "../../certs/cert.pem");
/// NOTE: this certificate is to be used for demonstration purposes only!
pub static KEY_PEM: &str = include_str!(
    "../../certs/key.pem"
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut server = Server::builder()
        .with_tls((CERT_PEM, KEY_PEM))?
        .with_io("0.0.0.0:4433")?
        .start()?;

    while let Some(connection) = server.accept().await {
        handle_connection(connection);
    }

    eprintln!("async fn main()");
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(())
}
fn handle_connection(conn: s2n_quic::Connection) {
    let mut connection = conn;
    tokio::spawn(async move {
        eprintln!(">> Connection accepted from {:?}", connection.remote_addr());
        while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
            let (mut recv_stream, mut send_stream) = stream.split();
            let (tx, mut rx) = mpsc::channel(32);
            let ping_tx = tx.clone();

            tokio::spawn(async move {
                let mut i = 1;
                loop {
                    let data: Bytes = Bytes::from(format!("{}\r\n", i));
                    if let Err(_) = ping_tx.send(data).await {
                        return;
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    i += 1;
                }
            });

            // spawn a new task for the stream
            tokio::spawn(async move {
                eprintln!("Stream opened from {:?}", recv_stream.connection().remote_addr());

                // echo any data back to the stream
                while let Ok(Some(data)) = recv_stream.receive().await {
                    if let Err(_) = tx.send(data).await {
                        break;
                    }
                }
                eprintln!("Stream closed from {:?}", recv_stream.connection().remote_addr());
                tx.send(Bytes::new()).await.expect("bytes::new()");
            });
            tokio::spawn(async move {
                while let Some(data) = rx.recv().await {
                    if data.is_empty() {
                        eprintln!("rx.close()");
                        rx.close();
                        break;
                    }
                    match String::from_utf8( data.to_vec() ) {
                        Ok(str) =>eprint!("send_stream {} {}", send_stream.connection().remote_addr().unwrap(), str ),
                        _ =>{}
                    }
                    //let _ = send_stream.poll_send_ready(&mut Context::default);
                    let _ = send_stream.send(data).await;
                }
            });
        }
        eprintln!("<< Connection accepted from {:?}", connection.remote_addr());
    });
}
