use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use skillratings::mhth::MhthRating;
use uuid::Uuid;

use crate::rpc::matchmaking::Player;

pub mod matchmaking {
    tonic::include_proto!("matchmaking");
}

pub mod helper;
pub mod server;

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct QueuedPlayer {
    pub player_id: Uuid,
    pub skillrating: MhthRating,
    pub region: String,
    pub ping: i32,
    pub difficulty: i32,
    pub join_mode: i32,
    pub party_mode: i32,
    pub party_ids: Vec<String>,
}

impl From<(Uuid, Player, MhthRating)> for QueuedPlayer {
    fn from(
        (player_id, player, skillrating): (Uuid, Player, skillratings::mhth::MhthRating),
    ) -> Self {
        Self {
            player_id,
            skillrating,
            ping: player.ping,
            difficulty: player.difficulty,
            join_mode: player.join_mode,
            region: player.region,
            party_mode: player.party_mode,
            party_ids: player.party_member_id,
        }
    }
}
