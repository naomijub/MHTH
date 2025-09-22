use std::time::Duration;

use chrono::{Local, NaiveDate};
use redis::AsyncTypedCommands;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Status};
use tracing::{debug, error};
use uuid::Uuid;

use super::matchmaking::matchmaking_service_server::MatchmakingService;
pub use super::matchmaking::matchmaking_service_server::MatchmakingServiceServer;
use crate::{
    nakama::{self, Authenticated},
    rpc::{
        helper::{time_since, IntoTonicError}, matchmaking::{HealthCheckRequest, HealthCheckResponse, JoinQueueResponse, Player}, QueuedPlayer
    },
};

pub mod healthcheck;

pub(crate) static GAME_START: Option<NaiveDate> = NaiveDate::from_yo_opt(2025, 1);

#[derive(Debug, Clone)]
pub struct MatchmakingServer {
    pub redis: redis::Client,
    pub http_client: reqwest::Client,
    pub nakama_client: nakama::NakamaClient<Authenticated>,
}

#[tonic::async_trait]
impl MatchmakingService for MatchmakingServer {
    type WatchStream = healthcheck::ResponseStream;

    async fn join_queue(
        &self,
        request: Request<Player>,
    ) -> Result<tonic::Response<JoinQueueResponse>, tonic::Status> {
        let player_id = Uuid::parse_str(&request.get_ref().player_id).to_tonic_error(
            format!("Invalid player id: {}", request.get_ref().player_id),
            Box::new(tonic::Status::invalid_argument),
        )?;
        let skillrating = self
            .nakama_client
            .get_skill_rating(&self.http_client, &request.get_ref().player_id)
            .await
            .inspect_err(|err| error!("Nakama API failed: {err}\n{err:?}"))
            .to_tonic_error("Nakama API failed", Box::new(tonic::Status::internal))?;
        let data: QueuedPlayer = (player_id, request.into_inner(), skillrating).into();
        let mut conn = self
            .redis
            .get_multiplexed_tokio_connection()
            .await
            .inspect_err(|err| error!("Redis failed to connect: {err}"))
            .to_tonic_error(
                "Redis failed to connect",
                Box::new(tonic::Status::unavailable),
            )?;
        conn.set(player_id, bitcode::encode(&data))
            .await
            .inspect_err(|err| error!("Redis failed to save player: {err}"))
            .to_tonic_error(
                format!("Failed to save player `{player_id}` to redis"),
                Box::new(tonic::Status::internal),
            )?;

        let player_queue_key = format!(
            "queue:{}:{}:{}",
            data.party_mode,
            data.party_ids.len(),
            data.region
        );

        let dt = Local::now();
        let order = conn.zadd(
            player_queue_key,
            bitcode::encode(&data),
            time_since(&dt)?,
        )
        .await
        .inspect_err(|err| error!("Redis failed to queue player: {err}\n{err:?}"))
        .to_tonic_error("Failed to add player to queue", Box::new(tonic::Status::internal))?;

        debug!("Player: `{player_id}` Index: {order}");
        Ok(tonic::Response::new(JoinQueueResponse {
            player_id: player_id.to_string(),
            status: "waiting in queue".to_string(),
        }))
    }

    async fn check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<tonic::Response<HealthCheckResponse>, tonic::Status> {
        Ok(tonic::Response::new(healthcheck::healthy(request)))
    }

    async fn watch(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<tonic::Response<Self::WatchStream>, tonic::Status> {
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
