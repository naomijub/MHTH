use redis::AsyncCommands;
use tracing::info;

use crate::rpc::{CLOSED_MATCHES, Match, worker::MatchmakingWorker};

impl MatchmakingWorker {
    pub async fn start_matches(&mut self) -> Result<usize, ()> {
        let mut count = 0;
        if let Ok(encoded_matchs) = &self
            .redis
            .zrange::<&str, Vec<Vec<u8>>>(CLOSED_MATCHES, 0, -1)
            .await
        {
            for (decoded_match, encoded) in encoded_matchs.iter().filter_map(|matches_bits| {
                Some((
                    bitcode::decode::<Match>(matches_bits.as_slice()).ok()?,
                    matches_bits,
                ))
            }) {
                self.redis
                    .zrem(CLOSED_MATCHES, encoded)
                    .await
                    .map(|_: ()| ())
                    .unwrap();
                info!("Call Nakama start match RPC: {decoded_match:?}");
                count += 1;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use redis::aio::MultiplexedConnection;
    use skillratings::mhth::MhthRating;
    use testcontainers::{
        ContainerAsync, GenericImage, ImageExt,
        core::{IntoContainerPort, WaitFor},
        runners::AsyncRunner,
    };
    use uuid::Uuid;

    use super::*;
    use crate::{
        nakama::{Authenticated, NakamaClient},
        rpc::{QueuedPlayer, create_match_queue_key, matchmaking::Player, player_queue_key},
    };

    #[tokio::test]
    async fn start_closed_matches() {
        let not_friend_id = Uuid::new_v4();
        let not_friend: QueuedPlayer = (
            not_friend_id,
            Player {
                join_mode: 2,
                region: "CAN".to_string(),
                ..Default::default()
            },
            MhthRating::default(),
        )
            .into();
        let friend_1_id = Uuid::new_v4();
        let friend_1: QueuedPlayer = (
            friend_1_id,
            Player {
                join_mode: 2,
                region: "CAN".to_string(),
                ..Default::default()
            },
            MhthRating::default(),
        )
            .into();
        let friend_2_id = Uuid::new_v4();
        let friend_2: QueuedPlayer = (
            friend_2_id,
            Player {
                join_mode: 2,
                region: "CAN".to_string(),
                ..Default::default()
            },
            MhthRating::default(),
        )
            .into();
        let friend_3_id = Uuid::new_v4();
        let friend_3: QueuedPlayer = (
            friend_3_id,
            Player {
                join_mode: 2,
                region: "CAN".to_string(),
                ..Default::default()
            },
            MhthRating::default(),
        )
            .into();
        let host_id = Uuid::new_v4();
        let player: QueuedPlayer = (
            host_id,
            Player {
                join_mode: 0,
                region: "CAN".to_string(),
                party_member_id: vec![
                    friend_2_id.to_string(),
                    friend_1_id.to_string(),
                    friend_3_id.to_string(),
                ],
                ..Default::default()
            },
            MhthRating::default(),
        )
            .into();
        let container = create_redis(6379).await;
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let client = redis_client(host.to_string(), port);
        let conn = client.get_multiplexed_async_connection().await.unwrap();
        init_regions(conn.clone()).await;
        let nakama = auth_client(666);
        // add players to queue
        for (score, p) in [
            player.clone(),
            not_friend,
            friend_2.clone(),
            friend_1.clone(),
            friend_3.clone(),
        ]
        .iter()
        .enumerate()
        {
            let encode = bitcode::encode(p);
            let key = player_queue_key(p);
            conn.clone()
                .set_ex(p.player_id, &encode, 200)
                .await
                .map(|_: ()| ())
                .unwrap();
            conn.clone()
                .zadd(key, encode, score)
                .await
                .map(|_: ()| ())
                .unwrap();
        }
        // set hosted match
        let create_match_key = create_match_queue_key(&player.region);
        let encoded_player = bitcode::encode(&player);
        conn.clone()
            .zadd(create_match_key, &encoded_player, 1)
            .await
            .map(|_: ()| ())
            .unwrap();
        let mut worker = MatchmakingWorker::new(
            conn.clone(),
            Arc::new(reqwest::Client::new()),
            nakama.into(),
        );
        worker.hosted_matches().await.unwrap();
        let matches = worker.start_matches().await.unwrap();

        container.pause().await.unwrap();

        assert_eq!(matches, 1)
    }

    async fn init_regions(conn: MultiplexedConnection) {
        let regions = &[
            "CAN".to_string(),
            "US".to_string(),
            "SOUTH_AMERICA".to_string(),
        ];

        crate::regions::set_regions(conn, regions).await.unwrap();
    }

    fn redis_client(host: String, port: u16) -> redis::Client {
        redis::Client::open(format!("redis://{host}:{port}")).unwrap()
    }

    async fn create_redis(port: u16) -> ContainerAsync<GenericImage> {
        GenericImage::new("redis", "8.2.1-bookworm")
            .with_exposed_port(port.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .with_env_var("REDIS_PASSWORD", "super-secret-password")
            .with_env_var("REDIS_USER", "redis_mms_admin")
            .start()
            .await
            .expect("Failed to start Redis")
    }

    pub fn auth_client(port: u16) -> NakamaClient<Authenticated> {
        NakamaClient {
            username: "username".to_string(),
            password: "password".to_string(),
            token: Some("super_random_token".to_string()),
            url: format!("http://127.0.0.1:{port}"),
            server_key_name: "defaultkey".to_string(),
            server_key_value: "server_key".to_string(),
            encryption_key: "encryption_key".to_string(),
            _state: std::marker::PhantomData::<Authenticated>,
        }
    }
}
