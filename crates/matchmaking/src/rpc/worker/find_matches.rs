use redis::{AsyncTypedCommands, RedisError};
use tracing::{error, info};

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
        let mut conn = self.redis.clone();
        let Some(regions) = conn.get(REGIONS_KEY).await? else {
            error!("No regions registred");
            return Ok(());
        };
        let regions: Vec<String> = bitcode::decode(regions.as_bytes())?;

        for region_key in regions.iter().map(create_match_queue_key) {
            while let Ok(host_players) = conn.zrange(&region_key, 0, -1).await {
                for player in host_players.iter().filter_map(|player_bit| {
                    bitcode::decode::<QueuedPlayer>(player_bit.as_bytes()).ok()
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
            }
        }
        if let Err(err) = self.remove_matched_players().await {
            error!("{err}");
        };

        let mut open_matches = Vec::new();

        for (index, a_match) in self.open_matches.iter().enumerate() {
            if a_match.players.len() >= 4 {
                if (conn.del(match_data_key(a_match)).await).is_ok() {
                    let encode = bitcode::encode(a_match);
                    conn.zadd(CLOSED_MATCHES, encode, index).await?;
                }
            } else {
                open_matches.push(a_match.clone());
            }
        }

        self.open_matches = open_matches;

        Ok(())
    }
}
