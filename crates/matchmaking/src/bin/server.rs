use std::str::FromStr;
use std::net::ToSocketAddrs;

use matchmaking::{internal_clients::InternalClients, nakama::NakamaClient, rpc::{
    server::{MatchmakingServer, MatchmakingServiceServer},
}};
use tonic::transport::Server;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = std::env::var("LOG_LEVEL")
        .ok()
        .and_then(to_log_level)
        .unwrap_or(tracing::Level::DEBUG);
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .try_init().unwrap();
    let clients = InternalClients::try_from_env()?;
    let nakama_client = NakamaClient::try_new()?
        .authenticate(clients.http_client()).await?;
    let matchmaking_server = MatchmakingServer{ 
        redis: clients.redis, 
        http_client: clients.http_client, 
        nakama_client 
    };

    let server = MatchmakingServiceServer::new(matchmaking_server);
    Server::builder()
        .add_service(server)
        .serve("0.0.0.0:50051".to_socket_addrs().unwrap().next().unwrap())
        .await?;
    Ok(())
}

fn to_log_level(env: String) -> Option<tracing::Level> {
    tracing::Level::from_str(&env.to_uppercase()).ok()
}