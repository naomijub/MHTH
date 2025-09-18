use std::time::Duration;

use redis::AsyncCommands;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Status};
use tracing::{debug, error};
use uuid::Uuid;

use crate::{nakama, rpc::{matchmaking::{Empty, HealthCheckRequest, HealthCheckResponse, Player}, server::healthcheck::ServingStatus, QueuedPlayer}};

use super::matchmaking::matchmaking_service_server::{MatchmakingService, SERVICE_NAME};

pub use super::matchmaking::matchmaking_service_server::MatchmakingServiceServer;

pub mod healthcheck;

#[derive(Debug, Clone)]
pub struct MatchmakingServer {
    pub redis: redis::Client,
    pub http_client: reqwest::Client,
    pub nakama_client: nakama::NakamaClient,
}

#[tonic::async_trait]
impl MatchmakingService for MatchmakingServer {
    type WatchStream = healthcheck::ResponseStream;

    async fn join_queue(&self, request: Request<Player>) -> Result<tonic::Response<Empty>, tonic::Status> {
        let player_id = Uuid::parse_str(&request.get_ref().player_id)
            .inspect_err(|err| error!("{:?}", err))
            .map_err(|_| tonic::Status::invalid_argument(format!("Invalid player id: {}", request.get_ref().player_id)))?;
        let skillrating = self.nakama_client.get_skill_rating(&self.http_client, &request.get_ref().player_id).await
            .inspect_err(|err| error!("{:?}", err))
            .map_err(|_| tonic::Status::internal("Nakama API failed"))?;
        let data: QueuedPlayer = (player_id, request.into_inner(), skillrating).into();
        let conn = self.redis.get_multiplexed_tokio_connection().await
            .inspect_err(|err| error!("{:?}", err))
            .map_err(|_| 
                tonic::Status::unavailable("Redis failed to connect")
            )?;
        
        Ok(tonic::Response::new(Empty {}))
    }

    async fn check(&self, request: Request<HealthCheckRequest>) -> Result<tonic::Response<HealthCheckResponse>, tonic::Status> {
        Ok(tonic::Response::new(healthcheck::healthy(request)))
    }

    async fn watch(&self, request: Request<HealthCheckRequest>) -> Result<tonic::Response<Self::WatchStream>, tonic::Status> {
        debug!("MatchmakingServer::watch::healthcheck");
        debug!("\tclient connected from: {:?}", request.remote_addr());

        // creating infinite stream with requested message
        let repeat = std::iter::repeat(healthcheck::healthy(request));
        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(200)));

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
                        break;
                    }
                }
            }
            debug!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);

        Ok(tonic::Response::new(
            Box::pin(output_stream) as Self::WatchStream
        ))
    }
}
