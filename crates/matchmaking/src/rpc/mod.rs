use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use skillratings::mhth::MhthRating;
use uuid::Uuid;

use crate::rpc::matchmaking::Player;

pub mod matchmaking {
    tonic::include_proto!("matchmaking");
}

pub mod helper;
pub mod player_impl;
pub mod server;
pub mod worker;

pub const CLOSED_MATCHES: &str = "matches:closed";
pub const PLAYER_QUEUE: &str = "queue_player";
pub const CREATE_MATCH_QUEUE: &str = "queue_create_match";

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Match {
    id: Uuid,
    players: Vec<QueuedPlayer>,
    region: String,
    host_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct QueuedPlayer {
    pub player_id: Uuid,
    pub skillrating: MhthRating,
    pub region: String,
    pub ping: i32,
    pub difficulty: i32,
    pub join_mode: i32,
    pub party_mode: i32,
    pub party_ids: Vec<String>,
    pub join_time: i64,
}

pub fn player_queue_key(data: &QueuedPlayer) -> String {
    format!("{PLAYER_QUEUE}:{}:{}", data.party_mode, data.region)
}

pub fn create_match_queue_key(region: &String) -> String {
    format!("{CREATE_MATCH_QUEUE}:{}", region)
}

pub fn match_data_key(new_match: &Match) -> String {
    format!("match:{}", new_match.id)
}
