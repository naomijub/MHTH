use redis::{AsyncCommands, RedisError};
use tracing::{error, info, warn};

use crate::{
    regions::REGIONS_KEY,
    rpc::{
        CLOSED_MATCHES, QueuedPlayer, create_match_queue_key, match_data_key,
        worker::MatchmakingWorker,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid player friend id: `{0}`")]
    InvalidFriendId(String),
    #[error(transparent)]
    Redis(#[from] RedisError),
    #[error(transparent)]
    BitcodeDeser(#[from] bitcode::Error),
}

impl MatchmakingWorker {
    pub async fn hosted_matches(&mut self) -> Result<(), Error> {
        let mut conn: redis::aio::MultiplexedConnection = self.redis.clone();
        let Some(regions): Option<Vec<u8>> = conn.get(REGIONS_KEY).await? else {
            error!("No regions registred");
            return Ok(());
        };
        let regions: Vec<String> = bitcode::decode(regions.as_slice())?;

        for region_key in regions.iter().map(create_match_queue_key) {
            if let Ok(host_players) = conn.zrange::<_, Vec<Vec<u8>>>(&region_key, 0, -1).await {
                for player in host_players.into_iter().filter_map(|player_bits| {
                    bitcode::decode::<QueuedPlayer>(player_bits.as_slice()).ok()
                }) {
                    match self.create_match(&player).await {
                        Ok(true) => info!("match created for player {}", player.player_id),
                        Ok(false) => error!("match not created for player {}", player.player_id),
                        Err(err) => error!(
                            "failed to create match for player {}: {err}",
                            player.player_id
                        ),
                    }
                }
            } else {
                warn!("Failed to find open matches for region {region_key}");
            }
        }

        if let Err(err) = self.remove_matched_players().await {
            error!("{err}");
        };

        let mut open_matches = Vec::new();

        for (index, a_match) in self.open_matches.iter().enumerate() {
            // TODO: Customize to player max expected okayers
            if a_match.players.len() >= 4 {
                if (conn.del(match_data_key(a_match)).await.map(|_: ()| ())).is_ok() {
                    let encode = bitcode::encode(a_match);
                    conn.zadd(CLOSED_MATCHES, encode, index)
                        .await
                        .map(|_: ()| ())?;
                } else {
                    error!(
                        "failed to add match `{}` to closed matches queue",
                        a_match.id
                    );
                }
            } else {
                open_matches.push(a_match.clone());
            }
        }

        self.open_matches = open_matches;

        Ok(())
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
        rpc::{Match, matchmaking::Player, player_queue_key},
    };

    #[tokio::test]
    async fn manage_hosted_matches_test() {
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
        let closed_matches = conn
            .clone()
            .zrange::<&str, Vec<Vec<u8>>>(CLOSED_MATCHES, 0, -1)
            .await
            .unwrap();
        container.pause().await.unwrap();

        assert_eq!(worker.open_matches, vec![]);
        assert_eq!(closed_matches.len(), 1);
        let closed_match: Match = bitcode::decode(closed_matches[0].as_slice()).unwrap();

        assert_eq!(closed_match.host_id, host_id);
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
