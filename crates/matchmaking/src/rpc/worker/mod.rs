use std::sync::Arc;

use crate::{
    nakama::{self, Authenticated},
    rpc::Match,
};

pub mod can_match;
pub mod find_matches;
pub mod form_match;
pub mod start_matches;

#[derive(Debug, Clone)]
pub struct MatchmakingWorker {
    pub redis: redis::aio::MultiplexedConnection,
    pub http_client: Arc<reqwest::Client>,
    pub nakama_client: Arc<nakama::NakamaClient<Authenticated>>,
    pub open_matches: Vec<Match>,
}

impl MatchmakingWorker {
    pub fn new(
        redis: redis::aio::MultiplexedConnection,
        http_client: Arc<reqwest::Client>,
        nakama_client: Arc<nakama::NakamaClient<Authenticated>>,
    ) -> Self {
        Self {
            redis,
            http_client,
            nakama_client,
            open_matches: Vec::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        self.hosted_matches().await.unwrap();
        self.start_matches().await.unwrap();

        Ok(())
    }
}
