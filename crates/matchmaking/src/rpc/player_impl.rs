use skillratings::mhth::MhthRating;
use uuid::Uuid;

use crate::rpc::{Player, QueuedPlayer};

impl QueuedPlayer {
    pub const fn joined_at(mut self, join_time: i64) -> Self {
        self.join_time = join_time;
        self
    }
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
            join_time: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_from_player() {
        let id = Uuid::new_v4();
        let queued: QueuedPlayer = (id, Player::default(), MhthRating::new()).into();

        assert_eq!(id, queued.player_id);
        assert_eq!(25., queued.skillrating.rating);
        assert_eq!(0, queued.ping);
    }
}
