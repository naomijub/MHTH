use std::{net::ToSocketAddrs, str::FromStr, sync::Arc};

use matchmaking::{
    internal_clients::InternalClients,
    nakama::NakamaClient,
    rpc::{
        server::{MatchmakingServer, MatchmakingServiceServer, auth::check_auth},
        worker::MatchmakingWorker,
    },
};
use tokio::time::{self, Duration};
use tonic::transport::Server;
use tracing::error;

const WORKER_EXECUTION_INTERVAL: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut interval = time::interval(WORKER_EXECUTION_INTERVAL);
    let log_level = std::env::var("LOG_LEVEL")
        .ok()
        .and_then(to_log_level)
        .unwrap_or(tracing::Level::DEBUG);
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .try_init()
        .unwrap();
    let clients = InternalClients::try_from_env()?;
    let nakama_client = Arc::new(
        NakamaClient::try_new()?
            .authenticate(clients.http_client())
            .await?,
    );
    let redis_conn = clients
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .inspect_err(|err| error!("Redis failed to connect: {err}"))?;
    let http_client = Arc::new(clients.http_client);
    let matchmaking_server = MatchmakingServer {
        redis: redis_conn.clone(),
        http_client: http_client.clone(),
        nakama_client: nakama_client.clone(),
    };
    let mut matchmaking_worker = MatchmakingWorker::new(redis_conn, http_client, nakama_client);

    tokio::spawn(async move {
        interval.tick().await;

        loop {
            interval.tick().await;
            if let Err(err) = matchmaking_worker.run().await {
                error!("matchmaking worker: {err:?}");
            }
        }
    });

    let server = MatchmakingServiceServer::with_interceptor(matchmaking_server, check_auth);
    Server::builder()
        .add_service(server)
        .serve("0.0.0.0:50051".to_socket_addrs().unwrap().next().unwrap())
        .await?;
    Ok(())
}

fn to_log_level(env: String) -> Option<tracing::Level> {
    tracing::Level::from_str(&env.to_uppercase()).ok()
}
