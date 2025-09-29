use bitcode::{Decode, Encode};
use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::rpc::{Match, QueuedPlayer, helper::time_since, matchmaking::JoinMode};

#[derive(Debug, Serialize, Deserialize, Encode, Decode, PartialEq, Eq)]
pub enum PingDeviation {
    /// Ping is less than 50 ms
    Excellent = 0,
    /// Ping is between 50 and 100 ms
    Good,
    /// Ping is between 100 and 150 ms
    Disadvantage,
    /// Ping is between 150 and 300 ms
    Poor,
    /// Ping is above 300+ ms - not playable
    Worst,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Player cannot host a match")]
    JoinOnlyMode,
    #[error("Party (`{count}`) is larger than MAX CAPACITY: {max}")]
    OversidedParty { count: usize, max: usize },
}

impl Match {
    const MAX_PLAYERS: usize = 4;

    pub fn host(player: &QueuedPlayer, party: &[QueuedPlayer]) -> Result<Self, Error> {
        let join_only_mode: i32 = JoinMode::JoinRoom.into();
        if player.join_mode == join_only_mode {
            return Err(Error::JoinOnlyMode);
        }
        if party.len() + 1 > Self::MAX_PLAYERS {
            return Err(Error::OversidedParty {
                count: party.len() + 1,
                max: Self::MAX_PLAYERS,
            });
        }
        let mut party = party.to_vec();
        party.push(player.clone());
        Ok(Self {
            host_id: player.player_id,
            id: Uuid::new_v4(),
            region: player.region.clone(),
            players: party,
        })
    }

    /// Can player be matched?
    pub fn is_player_fit(&self, player: QueuedPlayer) -> (bool, PingDeviation) {
        let current_players_count = self.players.len();
        let create_room: i32 = JoinMode::CreateRoom.into();
        if player.join_mode == create_room
            || current_players_count >= Self::MAX_PLAYERS
            || self.region != player.region
        {
            return (false, PingDeviation::Worst);
        }
        let average_ping = (self.players.iter().map(|p| p.ping).sum::<i32>() as f64)
            / (current_players_count as f64);
        let average_skill = (self
            .players
            .iter()
            .map(|p| p.skillrating.rating + p.skillrating.loadout_modifier)
            .sum::<f64>())
            / (current_players_count as f64);
        let player_skill = player.skillrating.rating + player.skillrating.loadout_modifier;

        let percent_skill = ((player_skill / average_skill) - 1f64) * 50f64;

        if player.ping < 50 {
            (true, PingDeviation::Excellent)
        } else if player.ping < 100 {
            (true, PingDeviation::Good)
        } else if player.ping < 150 && (average_ping + 25f64) > (player.ping as f64) {
            (true, PingDeviation::Disadvantage)
        } else if (player.ping < 150 && more_than_minutes(1, player.join_time))
            || ((player.ping as f64 + percent_skill) > 150f64)
        {
            (true, PingDeviation::Poor)
        } else if player.ping < 150 {
            (false, PingDeviation::Disadvantage)
        } else if player.ping >= 150 && player.ping < 300 && more_than_minutes(3, player.join_time)
        {
            (true, PingDeviation::Poor)
        } else {
            (false, PingDeviation::Worst)
        }
    }
}

pub fn more_than_minutes(minutes: i64, joined_at: i64) -> bool {
    let dt = Local::now();
    let Ok(time_since) = time_since(&dt) else {
        return false;
    };
    ((time_since - joined_at) / 60) > minutes
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use skillratings::mhth::MhthRating;

    use super::*;

    #[test]
    fn single_player_match() {
        let id = Uuid::new_v4();
        let player = demo_player(id, JoinMode::JoinOrCreateRoom);
        let a_match = Match::host(&player, &[]).unwrap();

        assert_eq!(a_match.host_id, id);
        assert_eq!(a_match.region, player.region);
        assert_eq!(a_match.players.len(), 1);
    }

    #[test]
    fn clan_match() {
        let id = Uuid::new_v4();
        let player = demo_player(id, JoinMode::CreateRoom);
        let a_match = Match::host(&player, &[player.clone(), player.clone()]).unwrap();

        assert_eq!(a_match.host_id, id);
        assert_eq!(a_match.region, player.region);
        assert_eq!(a_match.players.len(), 3);
    }

    #[test]
    fn full_match() {
        let id = Uuid::new_v4();
        let player = demo_player(id, JoinMode::CreateRoom);
        let a_match =
            Match::host(&player, &[player.clone(), player.clone(), player.clone()]).unwrap();

        assert_eq!(a_match.host_id, id);
        assert_eq!(a_match.region, player.region);
        assert_eq!(a_match.players.len(), 4);
    }

    #[test]
    fn oversided_match() {
        let id = Uuid::new_v4();
        let player = demo_player(id, JoinMode::CreateRoom);
        let err = Match::host(
            &player,
            &[
                player.clone(),
                player.clone(),
                player.clone(),
                player.clone(),
            ],
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Party (`5`) is larger than MAX CAPACITY: 4"
        );
    }

    #[test]
    fn join_only_mode_match() {
        let id = Uuid::new_v4();
        let player = demo_player(id, JoinMode::JoinRoom);
        let err = Match::host(&player, &[]).unwrap_err();

        assert_eq!(err.to_string(), "Player cannot host a match")
    }

    #[test]
    fn full_match_no_other_join() {
        let host_id = Uuid::new_v4();
        let player = demo_player(host_id, JoinMode::CreateRoom);

        let a_match = Match::host(
            &player,
            &[
                demo_player(Uuid::new_v4(), JoinMode::JoinRoom),
                demo_player(Uuid::new_v4(), JoinMode::JoinRoom),
                demo_player(Uuid::new_v4(), JoinMode::JoinRoom),
            ],
        )
        .unwrap();

        let val = a_match.is_player_fit(demo_player(Uuid::new_v4(), JoinMode::JoinRoom));

        assert!(!val.0);
        assert_eq!(val.1, PingDeviation::Worst);
    }

    #[test]
    fn is_fit_for_match() {
        let host_id = Uuid::new_v4();
        let player = demo_player(host_id, JoinMode::CreateRoom);

        let a_match = Match::host(
            &player,
            &[
                demo_player(Uuid::new_v4(), JoinMode::JoinRoom),
                demo_player(Uuid::new_v4(), JoinMode::JoinRoom),
            ],
        )
        .unwrap();

        let val = a_match.is_player_fit(demo_player(Uuid::new_v4(), JoinMode::JoinRoom));

        assert!(val.0);
        assert_eq!(val.1, PingDeviation::Excellent);

        let val = a_match.is_player_fit(demo_player(Uuid::new_v4(), JoinMode::CreateRoom));

        assert!(!val.0);
        assert_eq!(val.1, PingDeviation::Worst);

        // differente region
        let mut other = demo_player(Uuid::new_v4(), JoinMode::JoinRoom);
        other.region = "OTHER".to_string();
        let val = a_match.is_player_fit(other);

        assert!(!val.0);
        assert_eq!(val.1, PingDeviation::Worst);
    }

    #[test]
    fn different_pings_for_match() {
        let host_id = Uuid::new_v4();
        let player = demo_player(host_id, JoinMode::CreateRoom);

        let a_match = Match::host(
            &player,
            &[
                demo_player(Uuid::new_v4(), JoinMode::JoinRoom),
                demo_player(Uuid::new_v4(), JoinMode::JoinRoom),
            ],
        )
        .unwrap();

        let mut other = demo_player(Uuid::new_v4(), JoinMode::JoinRoom);
        other.ping = 51;
        let val = a_match.is_player_fit(other);

        assert!(val.0);
        assert_eq!(val.1, PingDeviation::Good);

        let mut other = demo_player(Uuid::new_v4(), JoinMode::JoinRoom);
        other.ping = 101;
        // Joined at time zero
        let val = a_match.is_player_fit(other);

        assert!(val.0);
        assert_eq!(val.1, PingDeviation::Poor);

        let mut other = demo_player(Uuid::new_v4(), JoinMode::JoinRoom);
        other.ping = 101;
        // just joined
        let dt = Local::now() - Duration::seconds(10);
        let join = time_since(&dt).unwrap();
        other.join_time = join;

        let val = a_match.is_player_fit(other);

        assert!(!val.0);
        assert_eq!(val.1, PingDeviation::Disadvantage);

        // High ping but very skillfull
        let mut other = demo_player(Uuid::new_v4(), JoinMode::JoinRoom);
        other.ping = 101;
        other.skillrating.rating = 5000f64;
        // just joined
        let dt = Local::now() - Duration::seconds(10);
        let join = time_since(&dt).unwrap();
        other.join_time = join;

        let val = a_match.is_player_fit(other);

        assert!(val.0);
        assert_eq!(val.1, PingDeviation::Poor);

        let mut other = demo_player(Uuid::new_v4(), JoinMode::JoinRoom);
        other.ping = 201;
        // Joined at time zero
        let val = a_match.is_player_fit(other);
        assert!(val.0);
        assert_eq!(val.1, PingDeviation::Poor);
    }

    fn demo_player(id: Uuid, join_mode: JoinMode) -> QueuedPlayer {
        QueuedPlayer {
            player_id: id,
            skillrating: MhthRating::default(),
            region: "CAN".to_string(),
            ping: 20,
            difficulty: 0,
            join_mode: join_mode.into(),
            party_mode: 1,
            party_ids: vec![String::new(), String::new()],
            join_time: 0,
        }
    }
}
