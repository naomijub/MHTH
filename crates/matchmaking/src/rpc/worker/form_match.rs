use std::str::FromStr;

use redis::{AsyncTypedCommands, RedisError};
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

            let Some(data) = conn.get(friend_id).await? else {
                continue;
            };
            let friend_data: QueuedPlayer = bitcode::decode(data.as_bytes())
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

        conn.set_ex(redis_match_data_key, &encode_match, TWO_HOURS)
            .await?;

        Ok(())
    }

    pub(crate) async fn remove_matched_players(&mut self) -> Result<(), Error> {
        let mut conn = self.redis.clone();
        for (key, player) in self
            .open_matches
            .iter()
            .flat_map(|mtc| mtc.players.iter())
            .map(|player| (player_queue_key(player), bitcode::encode(player)))
        {
            if let Err(err) = conn.zrem(key, player).await {
                error!("failed to remove matched player: {err}");
            };
        }

        Ok(())
    }
}
