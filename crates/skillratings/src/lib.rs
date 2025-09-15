#![deny(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used
)]
#![allow(
    // This is turned off because of the rating values in the structs
    clippy::module_name_repetitions,
    // "TrueSkill" shows up as a false positive otherwise
    clippy::doc_markdown,
    // Need to cast usizes to f64s where precision is not that important, also there seems to be no good alternative.
    clippy::cast_precision_loss,
)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod elo;
pub mod glicko;
pub mod glicko2;
pub mod glicko_boost;
pub mod sticko;
pub mod trueskill;
pub mod weng_lin;

/// The possible outcomes for a match: SUCCESSFUL, DRAW, FAILURE.
///
/// Note that this is always from the perspective of player one.
/// That means a win is a win for player one and a loss is a win for player two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Outcomes {
    /// Mission was successful, from team_one's perspective.
    SUCCESSFUL,
    /// Mission was failure, from team_one's perspective.
    FAILURE,
    /// A draw.
    DRAW,
}

impl Outcomes {
    #[must_use]
    /// Converts the outcome of the match into the points used in chess (1 = Win, 0.5 = Draw, 0 = Loss).
    ///
    /// Used internally in several rating algorithms, but some, like TrueSkill, have their own conversion.
    pub const fn to_chess_points(self) -> f64 {
        // Could set the visibility to crate level, but maybe someone has a use for it, who knows.
        match self {
            Self::SUCCESSFUL => 1.0,
            Self::DRAW => 0.5,
            Self::FAILURE => 0.0,
        }
    }
}

/// Outcome for a free-for-all match or a match that involves more than two teams.
///
/// Every team is assigned a rank, depending on their placement. The lower the rank, the better.
/// If two or more teams tie with each other, assign them the same rank.
///
/// For example: Team A takes 1st place, Team C takes 2nd place, Team B takes 3rd place,
/// and Teams D and E tie with each other and both take the 4th place.
/// In that case you would assign Team A = 1, Team B = 3, Team C = 2, Team D = 4, and Team E = 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MultiTeamOutcome(usize);

impl MultiTeamOutcome {
    #[must_use]
    #[inline]
    /// Makes a new `MultiTeamOutcome` from a given rank.
    pub const fn new(rank: usize) -> Self {
        Self(rank)
    }

    #[must_use]
    #[inline]
    /// Returns the rank that corresponds to this `MultiTeamOutcome`.
    pub const fn rank(self) -> usize {
        self.0
    }
}

impl From<usize> for MultiTeamOutcome {
    #[inline]
    fn from(v: usize) -> Self {
        Self(v)
    }
}

impl From<MultiTeamOutcome> for usize {
    #[inline]
    fn from(v: MultiTeamOutcome) -> Self {
        v.0
    }
}

/// Measure of player's skill.
///
/// 📌 _**Important note:**_ Please keep in mind that some rating systems use widely different scales for measuring ratings.
/// Please check out the documentation for each rating system for more information, or use `None` to always use default values.
///
/// Some rating systems might consider other values too (volatility, age, matches played etc.).
/// If that is the case, we will use the default values for those.
pub trait Rating {
    /// A single value for player's skill
    fn rating(&self) -> f64;
    /// A value for the uncertainty of a players rating.
    /// If the algorithm does not include an uncertainty value, this will return `None`.
    fn uncertainty(&self) -> Option<f64>;
    /// Initialise a `Rating` with provided score and uncertainty, if `None` use default.
    /// If the algorithm does not include an uncertainty value it will get dismissed.
    fn new(rating: Option<f64>, uncertainty: Option<f64>) -> Self;
}

/// Rating system for 1v1 matches.
///
/// 📌 _**Important note:**_ The RatingSystem Trait only implements the `rate` and `expected_score` functions.
/// Some rating systems might also implement additional functions (confidence interval, match quality, etc.) which you can only access by using those directly.
pub trait RatingSystem {
    #[cfg(feature = "serde")]
    /// Rating type rating system.
    type RATING: Rating + Copy + std::fmt::Debug + DeserializeOwned + Serialize;
    #[cfg(not(feature = "serde"))]
    /// Rating type rating system.
    type RATING: Rating + Copy + std::fmt::Debug;
    /// Config type for rating system.
    type CONFIG;
    /// Initialise rating system with provided config. If the rating system does not require a config, leave empty brackets.
    fn new(config: Self::CONFIG) -> Self;
    /// Calculate ratings for two players based on provided ratings and outcome.
    fn rate(
        &self,
        player_one: &Self::RATING,
        player_two: &Self::RATING,
        outcome: &Outcomes,
    ) -> (Self::RATING, Self::RATING);
    /// Calculate expected outcome of two players. Returns probability of player winning from 0.0 to 1.0.
    fn expected_score(&self, player_one: &Self::RATING, player_two: &Self::RATING) -> (f64, f64);
}

/// Rating system for rating periods.
///
/// 📌 _**Important note:**_ The RatingPeriodSystem Trait only implements the `rate` and `expected_score` functions.
/// Some rating systems might also implement additional functions which you can only access by using those directly.
pub trait RatingPeriodSystem {
    #[cfg(feature = "serde")]
    /// Rating type rating system.
    type RATING: Rating + Copy + std::fmt::Debug + DeserializeOwned + Serialize;
    #[cfg(not(feature = "serde"))]
    /// Rating type rating system.
    type RATING: Rating + Copy + std::fmt::Debug;
    /// Config type for rating system.
    type CONFIG;
    /// Initialise rating system with provided config. If the rating system does not require a config, leave empty brackets.
    fn new(config: Self::CONFIG) -> Self;
    /// Calculate ratings for a player based on provided list of opponents and outcomes.
    fn rate(&self, player: &Self::RATING, results: &[(Self::RATING, Outcomes)]) -> Self::RATING;
    /// Calculate expected scores for a player and a list of opponents. Returns probabilities of the player winning from 0.0 to 1.0.
    fn expected_score(&self, player: &Self::RATING, opponents: &[Self::RATING]) -> Vec<f64>;
}

/// Rating system for two teams.
///
/// 📌 _**Important note:**_ The TeamRatingSystem Trait only implements the `rate` and `expected_score` functions.
/// Some rating systems might also implement additional functions which you can only access by using those directly.
pub trait TeamRatingSystem {
    #[cfg(feature = "serde")]
    /// Rating type rating system.
    type RATING: Rating + Copy + std::fmt::Debug + DeserializeOwned + Serialize;
    #[cfg(not(feature = "serde"))]
    /// Rating type rating system.
    type RATING: Rating + Copy + std::fmt::Debug;
    /// Config type for rating system.
    type CONFIG;
    /// Initialise rating system with provided config. If the rating system does not require a config, leave empty brackets.
    fn new(config: Self::CONFIG) -> Self;
    /// Calculate ratings for two teams based on provided ratings and outcome.
    fn rate(
        &self,
        team_one: &[Self::RATING],
        team_two: &[Self::RATING],
        outcome: &Outcomes,
    ) -> (Vec<Self::RATING>, Vec<Self::RATING>);
    /// Calculate expected outcome of two teams. Returns probability of team winning from 0.0 to 1.0.
    fn expected_score(&self, team_one: &[Self::RATING], team_two: &[Self::RATING]) -> (f64, f64);
}

/// Rating system for more than two teams.
///
/// 📌 _**Important note:**_ The MultiTeamRatingSystem Trait only implements the `rate` and `expected_score` functions.
/// Some rating systems might also implement additional functions which you can only access by using those directly.
pub trait MultiTeamRatingSystem {
    #[cfg(feature = "serde")]
    /// Rating type rating system
    type RATING: Rating + Copy + std::fmt::Debug + DeserializeOwned + Serialize;
    #[cfg(not(feature = "serde"))]
    /// Rating type rating system
    type RATING: Rating + Copy + std::fmt::Debug;
    /// Config type for rating system.
    type CONFIG;
    /// Initialise rating system with provided config. If the rating system does not require a config, leave empty brackets.
    fn new(config: Self::CONFIG) -> Self;
    /// Calculate ratings for multiple teams based on provided ratings and outcome.
    fn rate(
        &self,
        teams_and_ranks: &[(&[Self::RATING], MultiTeamOutcome)],
    ) -> Vec<Vec<Self::RATING>>;
    /// Calculate expected outcome of multiple teams. Returns probability of team winning from 0.0 to 1.0.
    fn expected_score(&self, teams: &[&[Self::RATING]]) -> Vec<f64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcomes_to_chess_points() {
        assert!((Outcomes::SUCCESSFUL.to_chess_points() - 1.0).abs() < f64::EPSILON);
        assert!((Outcomes::DRAW.to_chess_points() - 0.5).abs() < f64::EPSILON);
        assert!((Outcomes::FAILURE.to_chess_points() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_multi_team_outcome() {
        let outcome = MultiTeamOutcome::new(1);
        assert_eq!(outcome.rank(), 1);
        assert_eq!(outcome, MultiTeamOutcome::from(1));
        assert_eq!(outcome, 1.into());
        assert_eq!(usize::from(MultiTeamOutcome::from(1)), 1);
    }

    #[test]
    fn test_derives() {
        let outcome = Outcomes::SUCCESSFUL;

        assert_eq!(outcome, outcome.clone());
        assert!(!format!("{outcome:?}").is_empty());

        let multi_team_outcome = MultiTeamOutcome::new(1);
        assert_eq!(multi_team_outcome, multi_team_outcome.clone());
        assert!(!format!("{multi_team_outcome:?}").is_empty());
        assert!(MultiTeamOutcome::new(1) < MultiTeamOutcome::new(2));
    }
}
