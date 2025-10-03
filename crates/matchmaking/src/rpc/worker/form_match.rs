use std::str::FromStr;

use redis::{AsyncCommands, RedisError};
use tracing::error;
use uuid::Uuid;

use crate::rpc::{
    self, Match, QueuedPlayer, match_data_key, matchmaking::JoinMode, player_queue_key,
    server::TWO_HOURS, worker::MatchmakingWorker,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid player friend id: `{0}`")]
    InvalidFriendId(String),
    #[error(transparent)]
    Redis(#[from] RedisError),
    #[error("failed to deserialize queued player")]
    BitcodeDeser,
    #[error(transparent)]
    CanMatch(#[from] rpc::worker::can_match::Error),
}

impl MatchmakingWorker {
    pub(crate) async fn create_match(&mut self, player: &QueuedPlayer) -> Result<bool, Error> {
        let create_room: i32 = JoinMode::CreateRoom.into();
        if player.join_mode != create_room {
            return Ok(false);
        }

        let mut conn = self.redis.clone();
        let mut party = Vec::new();
        for friend in &player.party_ids {
            let friend_id = Uuid::from_str(friend)
                .inspect_err(|err| {
                    error!(
                        "invalid friend id `{friend}` for player `{}`: {}",
                        player.player_id, err
                    )
                })
                .map_err(|_| Error::InvalidFriendId(friend.to_owned()))?;

            let Some(data): Option<Vec<u8>> = conn.get(friend_id).await? else {
                continue;
            };
            let friend_data: QueuedPlayer = bitcode::decode(&data)
                .inspect_err(|err| error!("{err}"))
                .map_err(|_| Error::BitcodeDeser)?;

            party.push(friend_data);
        }

        let hosted_match = Match::host(player, &party)?;

        self.open_matches.push(hosted_match.clone());

        if let Err(err) = self.form_match(hosted_match).await {
            error!("failed to create match {err}");
            Ok(false)
        } else {
            Ok(true)
        }
    }

    async fn form_match(&self, new_match: Match) -> Result<(), Error> {
        let encode_match = bitcode::encode(&new_match);
        let redis_match_data_key = match_data_key(&new_match);

        let mut conn = self.redis.clone();

        conn.set_ex(&redis_match_data_key, &encode_match, TWO_HOURS)
            .await
            .map(|_: ()| ())?;

        Ok(())
    }

    pub(crate) async fn remove_matched_players(&self) -> Result<(), Error> {
        let mut conn = self.redis.clone();
        for (key, player) in self
            .open_matches
            .iter()
            .flat_map(|mtc| mtc.players.iter())
            .map(|player| (player_queue_key(player), bitcode::encode(player)))
        {
            if let Err(err) = conn.zrem(key, player).await.map(|_: ()| ()) {
                error!("failed to remove matched player: {err}");
            };
        }

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

    use super::*;
    use crate::{
        nakama::{Authenticated, NakamaClient},
        rpc::matchmaking::Player,
    };

    #[tokio::test]
    async fn join_player_doesnt_start_match() {
        let player: QueuedPlayer = (
            Uuid::new_v4(),
            Player {
                join_mode: 1,
                ..Default::default()
            },
            MhthRating::default(),
        )
            .into();
        let container = create_redis(6379).await;
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let client = redis_client(host.to_string(), port).await;
        let conn = client.get_multiplexed_async_connection().await.unwrap();

        let mut worker = MatchmakingWorker::new(
            conn,
            Arc::new(reqwest::Client::new()),
            auth_client(666).into(),
        );

        let not_created = worker.create_match(&player).await.unwrap();

        container.pause().await.unwrap();
        assert!(!not_created)
    }

    #[tokio::test]
    async fn form_match_sets_redis_data() {
        let match_id = Uuid::new_v4();
        let host_player: QueuedPlayer =
            (Uuid::new_v4(), Player::default(), MhthRating::default()).into();
        let new_match = Match {
            id: match_id,
            host_id: host_player.player_id,
            players: vec![host_player.clone()],
            region: "CAN".to_string(),
        };
        let container = create_redis(6379).await;
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let client = redis_client(host.to_string(), port).await;
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        init_regions(conn.clone()).await;

        let worker = MatchmakingWorker::new(
            conn.clone(),
            Arc::new(reqwest::Client::new()),
            auth_client(666).into(),
        );
        let redis_match_data_key = match_data_key(&new_match);

        worker.form_match(new_match).await.unwrap();

        let stored: Vec<u8> = conn
            .get(redis_match_data_key)
            .await
            .map(|u: Vec<u8>| u)
            .unwrap();
        let empty_key: Result<Option<Vec<u8>>, RedisError> = conn.get("random-key").await;

        container.pause().await.unwrap();
        let decoded: Match = bitcode::decode(&stored).unwrap();

        assert_eq!(decoded.host_id, host_player.player_id);
        assert_eq!(decoded.id, match_id);
        assert_eq!(decoded.region, "CAN");
        assert_eq!(empty_key.unwrap(), None);
    }

    async fn redis_client(host: String, port: u16) -> redis::Client {
        redis::Client::open(format!("redis://{host}:{port}")).unwrap()
    }

    async fn create_redis(port: u16) -> ContainerAsync<GenericImage> {
        GenericImage::new("redis", "8.2.1-bookworm")
            .with_exposed_port(port.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .with_network("bridge")
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

    async fn init_regions(conn: MultiplexedConnection) {
        let regions = &[
            "CAN".to_string(),
            "US".to_string(),
            "SOUTH_AMERICA".to_string(),
        ];

        crate::regions::set_regions(conn, regions).await.unwrap();
    }
}
