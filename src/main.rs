
use anyhow::Error;
use lofi_party::ws_conn::{self, handle_connection};


#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    
    log::info!("Starting Lofi Party server...");

    let server = ws_conn::create_websocket_connection().await?;

    while let Ok((stream, addr)) = server.accept().await {
        let message = handle_connection(stream, addr).await?;

        let event_details = message.to_text()?.to_string().split(":").map(|s| s.to_string()).collect::<Vec<String>>();

        //this tells us which action is performed, is it play, pause, skip, user joined, user left, new lobby created etc
        let action_type = event_details[0].as_str();
    }
    Ok(())
}
