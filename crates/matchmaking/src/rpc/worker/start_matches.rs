use redis::AsyncTypedCommands;
use tracing::info;

use crate::rpc::{CLOSED_MATCHES, Match, worker::MatchmakingWorker};

impl MatchmakingWorker {
    pub async fn start_matches(&mut self) -> Result<(), ()> {
        if let Ok(encoded_matchs) = &self.redis.zrange(CLOSED_MATCHES, 0, -1).await {
            for (decoded_match, encoded) in encoded_matchs.iter().filter_map(|matches_bits| {
                Some((
                    bitcode::decode::<Match>(matches_bits.as_bytes()).ok()?,
                    matches_bits,
                ))
            }) {
                self.redis.zrem(CLOSED_MATCHES, encoded).await.unwrap();
                info!("Call Nakama start match RPC: {decoded_match:?}")
            }
        }

        Ok(())
    }
}
