use std::{sync::Arc, time::Duration};

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
        QueuedPlayer, create_match_queue_key,
        helper::{IntoTonicError, time_since},
        matchmaking::{
            HealthCheckRequest, HealthCheckResponse, JoinMode, JoinQueueResponse, Player,
        },
        player_queue_key,
    },
};

pub mod auth;
pub mod healthcheck;

pub(crate) static TEN_MINUTES: u64 = 600;
pub(crate) static TWO_HOURS: u64 = 720;
pub(crate) static GAME_START: Option<NaiveDate> = NaiveDate::from_yo_opt(2025, 1);

#[derive(Debug, Clone)]
pub struct MatchmakingServer {
    pub redis: redis::aio::MultiplexedConnection,
    pub http_client: Arc<reqwest::Client>,
    pub nakama_client: Arc<nakama::NakamaClient<Authenticated>>,
}

#[tonic::async_trait]
impl MatchmakingService for MatchmakingServer {
    type WatchStream = healthcheck::ResponseStream;

    async fn join_queue(
        &self,
        request: Request<Player>,
    ) -> Result<tonic::Response<JoinQueueResponse>, tonic::Status> {
        let user_id = request.extensions().get::<auth::UserId>();

        let player_id = Uuid::parse_str(&request.get_ref().player_id).to_tonic_error(
            format!("Invalid player id: {}", request.get_ref().player_id),
            Box::new(tonic::Status::invalid_argument),
        )?;
        if user_id.is_none_or(|id| id.player_id != player_id.to_string()) {
            return Err(tonic::Status::unauthenticated("invalid player token"));
        }

        let skill_result = {
            let nakama_client = self.nakama_client.clone();
            let http_client = self.http_client.clone();
            nakama_client
                .get_skill_rating(http_client, &request.get_ref().player_id)
                .await
        };
        let skillrating = skill_result
            .inspect_err(|err| error!("Nakama API failed: {err}\n{err:?}"))
            .to_tonic_error("Nakama API failed", Box::new(tonic::Status::internal))?;
        let dt = Local::now();
        let time_since = time_since(&dt)?;
        let data: QueuedPlayer = (player_id, request.into_inner(), skillrating).into();
        let data = data.joined_at(time_since);

        // Redis block
        if cfg!(not(test)) {
            let encoded_player = bitcode::encode(&data);
            let mut conn = self.redis.clone();
            conn.set_ex(player_id, &encoded_player, TEN_MINUTES)
                .await
                .inspect_err(|err| error!("Redis failed to save player: {err}"))
                .to_tonic_error(
                    format!("Failed to save player `{player_id}` to redis"),
                    Box::new(tonic::Status::internal),
                )?;

            let player_key = player_queue_key(&data);

            let order = conn
                .zadd(player_key, &encoded_player, time_since)
                .await
                .inspect_err(|err| error!("Redis failed to queue player: {err}\n{err:?}"))
                .to_tonic_error(
                    "Failed to add player to queue",
                    Box::new(tonic::Status::internal),
                )?;
            debug!("Player: `{player_id}` Index: `{order}` TimeSince: `{time_since}`");

            let create_room: i32 = JoinMode::CreateRoom.into();
            if data.join_mode == create_room {
                let create_match_key = create_match_queue_key(&data.region);

                let _ = conn
                    .zadd(create_match_key, &encoded_player, time_since)
                    .await
                    .inspect_err(|err| {
                        error!("Redis failed to queue room creation: {err}\n{err:?}")
                    });
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_join_queue() {
        let clients = crate::internal_clients::InternalClients::try_from_env().unwrap();
        let nakama_client = nakama::NakamaClient::try_new()
            .unwrap()
            .authenticate(clients.http_client())
            .await
            .unwrap();
        let nakama_client = Arc::new(nakama_client);
        let http_client = Arc::new(clients.http_client);
        let matchmaking_server = MatchmakingServer {
            redis: clients
                .redis
                .get_multiplexed_tokio_connection()
                .await
                .unwrap(),
            http_client,
            nakama_client,
        };

        let response = matchmaking_server
            .join_queue(Request::new(Player {
                player_id: "01997433-3000-7b4b-8712-9253d26a68c8".to_string(),
                loadout_config: "".to_string(),
                region: "CAN".to_string(),
                ping: 20,
                difficulty: 1,
                join_mode: 2,
                party_mode: 0,
                party_member_id: Vec::new(),
            }))
            .await
            .unwrap();

        let response = response.into_inner();
        assert_eq!(response.player_id, "01997433-3000-7b4b-8712-9253d26a68c8");
        assert_eq!(response.status, "waiting in queue");
    }
}
