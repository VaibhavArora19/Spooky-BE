use tokio::{net::{TcpListener, TcpStream}};
use tokio_tungstenite::tungstenite::Message;
use std::net::SocketAddr;
use futures_util::stream::{StreamExt};

use crate::config;

pub async fn create_websocket_connection() -> Result<TcpListener, anyhow::Error> {
    let port = config::Config::get_config().port;

    log::info!("Starting WebSocket server on port: {}", port);

    let server = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| {
            log::error!(
                "Error creating a TCP Connection. Failed with error: {:?}",
                e
            );
            anyhow::Error::msg("Failed to connect to server")
        })?;

    Ok(server)
}


pub async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) -> Result<Message, anyhow::Error>{
    println!("Incoming TCP connection from: {:?}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream).await.expect("Error during the websocket handshake occured");

    log::info!("WebSocket connection established: {:?}", addr);

    let (_, mut incoming) = ws_stream.split();


    let broadcast_message_option = incoming.next().await;

        if broadcast_message_option.is_none() {
            log::error!("No message received from client: {:?}", addr);
            
            return Err(anyhow::Error::msg("No message received from client"));
        }

    let broadcast_message = broadcast_message_option.unwrap().map_err(|e| {
        log::info!("Failed to get broadcasted message. Failed with error: {:?}", e);

        anyhow::Error::msg("Failed to get broadcasted message")
    })?;

    log::info!("Received a message from {}: {:?}", addr, broadcast_message);


    Ok(broadcast_message)
}

