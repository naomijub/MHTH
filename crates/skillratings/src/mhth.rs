#![allow(clippy::float_cmp)]
//! A bayesian approximation method for online ranking. Similar to TrueSkill and Weng-Lin-Julia, but based on a logistical distribution and rating modifier.
//!
//! Developed by Ruby C. Weng, Chih-Jen Lin and Julia Naomi.
//! This algorithm is also known online as "MhthSkill", in reference to the TrueSkill and the Weng-Lin-Julia algorithms.
//! But the proper name would be `A Bayesian Approximation Method for Online Ranking with Statical Rating Modifiers`.
//!
//! Developed specifically for PVE online games with multiple teams and multiple players playing against the environment,
//! this algorithm aims to be simpler and faster (~2.5 - 6.5x) than TrueSkill while yielding similar accuracy.
//!
//! While TrueSkill is based upon a Gaussian distribution, this algorithm is based upon a logistical distribution, the Bradley-Terry model.
//!
//! # Quickstart
//!
//! This is the most basic example on how to use the Weng-Lin-Julia Module.
//! Please take a look at the functions below to see more advanced use cases.
//!
//! ```rust
//! use skillratings::{
//!     Outcomes,
//!     mhth::{MhthConfig, MhthRating, mhth},
//! };
//!
//! // Initialise a new player rating with a rating of 25, loadout_modifier of 1 and an uncertainty of 25/3 ≈ 8.33.
//! let player = MhthRating::new();
//!
//! // Or you can initialise it with your own values of course.
//! // Imagine these numbers being pulled from a database.
//! let (some_rating, some_loadout_modifier, some_uncertainty) = (41.2, 1.34, 2.12);
//! let environment = MhthRating {
//!     rating: some_rating,
//!     loadout_modifier: some_loadout_modifier,
//!     uncertainty: some_uncertainty,
//! };
//!
//! // The outcome of the match is from the perspective of player one.
//! let outcome = Outcomes::SUCCESSFUL;
//!
//! // The config allows you to specify certain values in the Weng-Lin-Julia calculation.
//! // Here we change the beta value from the default of 25 / 6 ≈ 4.167.
//! // The beta value measures the difference you need in rating points
//! // to achieve a ~67% win-rate over the environment.
//! // Lower this value if your game is heavily reliant on pure skill,
//! // or increase it if randomness plays a big factor in the outcome of the game.
//! // For more information on how to customise the config,
//! // please check out the MhthConfig struct.
//! let config = MhthConfig {
//!     beta: 25.0 / 12.0,
//!     ..Default::default()
//! };
//!
//! // The `mhth` function will calculate the new ratings for both players and return them.
//! let (new_player_one, new_environment) = mhth(&player, &environment, &outcome, &config);
//! ```
//!
//! # More Information
//! - [Original Paper (PDF)](https://jmlr.csail.mit.edu/papers/volume12/weng11a/weng11a.pdf)
//! - [Bradley-Terry model Wikipedia](https://en.wikipedia.org/wiki/Bradley–Terry_model)
//! - [Approximate Bayesian computation Wikipedia](https://en.wikipedia.org/wiki/Approximate_Bayesian_computation)
//! - [Logistic distribution Wikipedia](https://en.wikipedia.org/wiki/Logistic_distribution)
//! - [OpenSkill (Python Package)](https://openskill.me/en/stable/)

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    MultiTeamOutcome, MultiTeamRatingSystem, Outcomes, Rating, RatingPeriodSystem, RatingSystem,
    TeamRatingSystem, trueskill::TrueSkillRating,
};
use std::cmp::Ordering;

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// The Weng-Lin-Julia rating of a player.
///
/// Similar to [`TrueSkillRating`].
///
/// The default rating is 25.0.
/// The default loadout_modifier is 1.0.
/// The default uncertainty is 25/3 ≈ 8.33.
pub struct MhthRating {
    /// The rating value (mu) of the MhthRating, by default 25.0.
    pub rating: f64,
    /// The loadout modifier of the MhthRating, by default 1.0.
    pub loadout_modifier: f64,
    /// The uncertainty value (sigma) of the MhthRating, by default 25/3 ≈ 8.33
    /// To manually calculate this consider `sigma = mu / z`, where z is usually 3.
    pub uncertainty: f64,
}

impl MhthRating {
    #[must_use]
    /// Initialise a new MhthRating with a rating of 25.0, and an uncertainty of 25/3 ≈ 8.33.
    pub const fn new() -> Self {
        Self {
            rating: 25.0,
            loadout_modifier: 1.0,
            uncertainty: 25.0 / 3.0,
        }
    }
}

impl Default for MhthRating {
    fn default() -> Self {
        Self::new()
    }
}

impl MhthRating {
    /// Returns the unmodified rating value of the MhthRating.
    /// rating wiithout the loadout modifier.
    #[must_use]
    pub const fn rating_unmod(&self) -> f64 {
        self.rating
    }

    /// Sets the loadout modifier of the MhthRating.
    #[must_use]
    pub const fn loadout_modifier(mut self, modifier: f64) -> Self {
        self.loadout_modifier = modifier;
        self
    }
}

impl Rating for MhthRating {
    /// Returns the rating value of the MhthRating with the loadout modifier.
    fn rating(&self) -> f64 {
        self.rating + self.loadout_modifier
    }
    fn uncertainty(&self) -> Option<f64> {
        Some(self.uncertainty)
    }
    fn new(rating: Option<f64>, uncertainty: Option<f64>) -> Self {
        Self {
            rating: rating.unwrap_or(25.0),
            loadout_modifier: 1.0,
            uncertainty: uncertainty.unwrap_or(25.0 / 3.0),
        }
    }
}

impl From<(f64, f64)> for MhthRating {
    fn from((r, u): (f64, f64)) -> Self {
        Self {
            rating: r,
            loadout_modifier: 1.0,
            uncertainty: u,
        }
    }
}

impl From<(f64, f64, f64)> for MhthRating {
    fn from((r, m, u): (f64, f64, f64)) -> Self {
        Self {
            rating: r,
            loadout_modifier: m,
            uncertainty: u,
        }
    }
}

impl From<TrueSkillRating> for MhthRating {
    fn from(t: TrueSkillRating) -> Self {
        Self {
            rating: t.rating,
            loadout_modifier: 1.0,
            uncertainty: t.uncertainty,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// Constants used in the Weng-Lin-Julia calculations.
pub struct MhthConfig {
    /// The skill-class width, aka the number of difference in rating points
    /// needed to have a ~67% win probability against another player.
    /// By default set to 25 / 6 ≈ `4.167`.
    /// If your game is more reliant on pure skill, decrease this value,
    /// if there are more random factors, increase it.
    pub beta: f64,
    /// The lower ceiling of the sigma value, in the uncertainty calculations.
    /// The lower this value, the lower the possible uncertainty values.
    /// By default set to 0.000_001.
    /// Do not set this to a negative value.
    // `epsilon`
    pub uncertainty_tolerance: f64,
}

impl MhthConfig {
    #[must_use]
    /// Initialise a new `MhthConfig` with a beta value of 25 / 6 ≈ `4.167`
    /// and an uncertainty tolerance of `0.000_001`.
    pub fn new() -> Self {
        Self {
            beta: 25.0 / 6.0,
            uncertainty_tolerance: 0.000_001,
        }
    }
}

impl Default for MhthConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct to calculate ratings and expected score for [`MhthRating`]
pub struct Mhth {
    config: MhthConfig,
}

impl RatingSystem for Mhth {
    type RATING = MhthRating;
    type CONFIG = MhthConfig;

    fn new(config: Self::CONFIG) -> Self {
        Self { config }
    }

    fn rate(
        &self,
        player: &MhthRating,
        environment: &MhthRating,
        outcome: &Outcomes,
    ) -> (MhthRating, MhthRating) {
        mhth(player, environment, outcome, &self.config)
    }

    fn expected_score(&self, player: &MhthRating, environment: &MhthRating) -> (f64, f64) {
        expected_score(player, environment, &self.config)
    }
}

impl RatingPeriodSystem for Mhth {
    type RATING = MhthRating;
    type CONFIG = MhthConfig;

    fn new(config: Self::CONFIG) -> Self {
        Self { config }
    }

    fn rate(&self, player: &MhthRating, results: &[(MhthRating, Outcomes)]) -> MhthRating {
        mhth_rating_period(player, results, &self.config)
    }

    fn expected_score(&self, player: &Self::RATING, opponents: &[Self::RATING]) -> Vec<f64> {
        expected_score_rating_period(player, opponents, &self.config)
    }
}

impl TeamRatingSystem for Mhth {
    type RATING = MhthRating;
    type CONFIG = MhthConfig;

    fn new(config: Self::CONFIG) -> Self {
        Self { config }
    }

    fn rate(
        &self,
        team_one: &[MhthRating],
        team_two: &[MhthRating],
        outcome: &Outcomes,
    ) -> (Vec<MhthRating>, Vec<MhthRating>) {
        mhth_team_vs_environment(team_one, team_two, outcome, &self.config)
    }

    fn expected_score(&self, team_one: &[Self::RATING], team_two: &[Self::RATING]) -> (f64, f64) {
        expected_team_vs_environment(team_one, team_two, &self.config)
    }
}

impl MultiTeamRatingSystem for Mhth {
    type RATING = MhthRating;
    type CONFIG = MhthConfig;

    fn new(config: Self::CONFIG) -> Self {
        Self { config }
    }

    fn rate(
        &self,
        teams_and_ranks: &[(&[Self::RATING], MultiTeamOutcome)],
    ) -> Vec<Vec<MhthRating>> {
        mhth_multi_team(teams_and_ranks, &self.config)
    }

    fn expected_score(&self, teams: &[&[Self::RATING]]) -> Vec<f64> {
        expected_score_multi_team(teams, &self.config)
    }
}

#[must_use]
/// Calculates the [`MhthRating`]s of single player vs environment based on their old ratings, uncertainties, loadout_modifiers and the outcome of the game.
///
/// Takes in a player as [`MhthRating`]s, the environment as [`MhthRating`], an [`Outcome`](Outcomes), and a [`MhthConfig`].
///
/// The outcome of the match is in the perspective of the `player`.
/// This means [`Outcomes::SUCCESSFUL`] is a win for the `player` and [`Outcomes::FAILURE`] is a win for `environment`.
///
/// Similar to [`mhth_rating_period`] and [`mhth_team_vs_environment`].
///
/// > Good for single player with one mission.
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::{
///     Outcomes,
///     mhth::{MhthConfig, MhthRating, mhth},
/// };
///
/// let player = MhthRating {
///     rating: 42.0,
///     loadout_modifier: 3.0,
///     uncertainty: 1.3,
/// };
/// let environment = MhthRating::new();
///
/// let (new_player, new_environment) = mhth(
///     &player,
///     &environment,
///     &Outcomes::SUCCESSFUL,
///     &MhthConfig::new(),
/// );
///
/// assert_eq_float!((new_player.rating * 100.0).round(), 4202.0); // 4203.0 for openskill
/// assert_eq_float!((new_player.uncertainty * 100.0).round(), 130.0);
/// assert_eq_float!((new_environment.rating * 100.0).round(), 2415.0); // 2391.0 for openskill
/// assert_eq_float!((new_environment.uncertainty * 100.0).round(), 809.0); // 803.0 for openskill
/// ```
pub fn mhth(
    player: &MhthRating,
    environment: &MhthRating,
    outcome: &Outcomes,
    config: &MhthConfig,
) -> (MhthRating, MhthRating) {
    let c = 2.0f64
        .mul_add(
            config.beta.powi(2),
            player
                .uncertainty
                .mul_add(player.uncertainty, environment.uncertainty.powi(2)),
        )
        .sqrt();

    let (p1, p2) = p_value(
        player.rating + player.loadout_modifier,
        environment.rating,
        c,
    );

    let outcome1 = outcome.to_chess_points();
    let outcome2 = 1.0 - outcome1;

    let new_rating1 = new_rating(
        player.rating + player.loadout_modifier,
        player.uncertainty,
        c,
        p1,
        outcome1,
    ) - player.loadout_modifier;
    let new_rating2 = new_rating(
        environment.rating + environment.loadout_modifier,
        environment.uncertainty,
        c,
        p2,
        outcome2,
    ) - environment.loadout_modifier;

    let new_uncertainty1 = new_uncertainty(player.uncertainty, c, p1, config.uncertainty_tolerance);
    let new_uncertainty2 =
        new_uncertainty(environment.uncertainty, c, p2, config.uncertainty_tolerance);

    (
        MhthRating {
            rating: new_rating1,
            loadout_modifier: player.loadout_modifier,
            uncertainty: new_uncertainty1,
        },
        MhthRating {
            rating: new_rating2,
            loadout_modifier: environment.loadout_modifier,
            uncertainty: new_uncertainty2,
        },
    )
}

#[must_use]
/// Calculates a [`MhthRating`] in a non-traditional way using a rating period,
/// for compatibility with the other algorithms.
///
/// Takes in a player as an [`MhthRating`] and their results as a Slice of tuples containing the opponent as an [`MhthRating`],
/// the outcome of the game as an [`Outcome`](Outcomes) and a [`MhthConfig`].
///
/// The outcome of the match is in the perspective of the player.
/// This means [`Outcomes::SUCCESSFUL`] is a win for the player and [`Outcomes::FAILURE`] is a win for the opponent.
///
/// Similar to [`mhth`] or [`mhth_team_vs_environment`].
///
/// > Good for single player with multiple missions.
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::{
///     Outcomes,
///     mhth::{MhthConfig, MhthRating, mhth_rating_period},
/// };
///
/// let player = MhthRating::new();
///
/// let environment_one = MhthRating::new();
/// let environment_two = MhthRating {
///     rating: 12.0,
///     loadout_modifier: 3.0,
///     uncertainty: 4.2,
/// };
///
/// let new_player_rating = mhth_rating_period(
///     &player,
///     &vec![
///         (environment_one, Outcomes::SUCCESSFUL),
///         (environment_two, Outcomes::DRAW),
///     ],
///     &MhthConfig::new(),
/// );
///
/// assert_eq_float!((new_player_rating.rating * 100.0).round(), 2678.0); // 2578.0 for openskill
/// assert_eq_float!((new_player_rating.uncertainty * 100.0).round(), 779.0); // 780.0 for openskill
/// ```
pub fn mhth_rating_period(
    player: &MhthRating,
    results: &[(MhthRating, Outcomes)],
    config: &MhthConfig,
) -> MhthRating {
    let mut player_rating = player.rating + player.loadout_modifier;
    let mut player_uncertainty = player.uncertainty;

    for (opponent, result) in results {
        let c = 2.0f64
            .mul_add(
                config.beta.powi(2),
                player_uncertainty.mul_add(player_uncertainty, opponent.uncertainty.powi(2)),
            )
            .sqrt();

        let (p, _) = p_value(
            player_rating + player.loadout_modifier,
            opponent.rating + opponent.loadout_modifier,
            c,
        );
        let outcome = result.to_chess_points();

        player_rating = new_rating(
            player_rating + player.loadout_modifier,
            player_uncertainty,
            c,
            p,
            outcome,
        ) - player.loadout_modifier;
        player_uncertainty =
            new_uncertainty(player_uncertainty, c, p, config.uncertainty_tolerance);
    }

    MhthRating {
        rating: player_rating,
        loadout_modifier: player.loadout_modifier,
        uncertainty: player_uncertainty,
    }
}

#[must_use]
/// Calculates the [`MhthRating`] of a team based on the players their ratings and uncertainties, the environment "team" rating, and the outcome of the game.
///
/// Takes in the team as a Slice of [`MhthRating`]s, the environment "team" as a Slice of [`MhthRating`]s, the outcome of the game as an [`Outcome`](Outcomes) and a [`MhthConfig`].
///
/// The outcome of the match is in the perspective of `team`.
/// This means [`Outcomes::SUCCESSFUL`] is a win for `team` and [`Outcomes::FAILURE`] is a win for `environment`.
///
/// Similar to [`mhth`].
///
/// > Typical for a team vs environment.
/// > - Environment can consist of a single entity, like a boss or a whole team of entities.
/// > - Usually good to average subentities groups in the process:
/// >   - Environment has 1 Boss with rating 200, rating is 200.
/// >   - 12 Drones with rating 50, rating is 12√(50 * 12)
/// >   - 4 Bots with rating 42, rating is 4√(42 * 4)
///
/// ## Info
/// When environment has a higher ranking than players combined,
/// then players win, means that higher loadout modifiers will reduce the amount that rating could increase.
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::{
///     Outcomes,
///     mhth::{MhthConfig, MhthRating, mhth_team_vs_environment},
/// };
///
/// let players_team = vec![
///     MhthRating::new(),
///     MhthRating {
///         rating: 30.0,
///         loadout_modifier: 3.0,
///         uncertainty: 1.2,
///     },
///     MhthRating {
///         rating: 21.0,
///         loadout_modifier: 3.2,
///         uncertainty: 6.5,
///     },
/// ];
///
/// let environment_team = vec![
///     MhthRating::default(),
///     MhthRating {
///         rating: 41.0,
///         loadout_modifier: 5.0,
///         uncertainty: 1.4,
///     },
///     MhthRating {
///         rating: 19.2,
///         loadout_modifier: 0.03,
///         uncertainty: 4.3,
///     },
/// ];
///
/// let (new_team, new_environment) = mhth_team_vs_environment(
///     &players_team,
///     &environment_team,
///     &Outcomes::SUCCESSFUL,
///     &MhthConfig::new(),
/// );
/// // originally 25 + loadout 1, increased to 27.9, 27.9 for openskill
/// assert_eq_float!((new_team[0].rating * 100.0).round(), 2783.0); // 2790.0 for openskill
/// // originlly 30  + loadout 3.0, increased to 30.06
/// assert_eq_float!((new_team[1].rating * 100.0).round(), 3006.0);
/// // originally 21 + loadout 3.2, increased to 22.72, 22.72 for openskill
/// assert_eq_float!((new_team[2].rating * 100.0).round(), 2272.0); // 2277.0 for openskill
///
/// // originally 25 + loadout 1, decreased to 22.17, 22.10 for openskill
/// assert_eq_float!((new_environment[0].rating * 100.0).round(), 2217.0); // 2210.0 for openskill
/// // originally 41 + loadout 5.0, decreased to 40.92
/// assert_eq_float!((new_environment[1].rating * 100.0).round(), 4092.0);
/// // originally 19.2 + loadout 0.03, decreased to 18.45, 18.43 for openskill
/// assert_eq_float!((new_environment[2].rating * 100.0).round(), 1845.0); // 1843.0 for openskill
/// ```
pub fn mhth_team_vs_environment(
    players_team: &[MhthRating],
    environment: &[MhthRating],
    outcome: &Outcomes,
    config: &MhthConfig,
) -> (Vec<MhthRating>, Vec<MhthRating>) {
    if players_team.is_empty() || environment.is_empty() {
        return (players_team.to_vec(), environment.to_vec());
    }

    let players_rating: f64 = players_team
        .iter()
        .map(|p| p.rating + p.loadout_modifier)
        .sum();
    let environment_rating: f64 = environment
        .iter()
        .map(|p| p.rating + p.loadout_modifier)
        .sum();

    let players_uncertainty_sq: f64 = players_team.iter().map(|p| p.uncertainty.powi(2)).sum();
    let environment_uncertainty_sq: f64 = environment.iter().map(|p| p.uncertainty.powi(2)).sum();

    let c = 2.0f64
        .mul_add(
            config.beta.powi(2),
            players_uncertainty_sq + environment_uncertainty_sq,
        )
        .sqrt();

    let (p1, p2) = p_value(players_rating, environment_rating, c);

    let outcome1 = outcome.to_chess_points();
    let outcome2 = 1.0 - outcome1;

    // Small delta is equivalent to omega as there are only two teams.
    let players_small_delta = small_delta(players_uncertainty_sq, c, p1, outcome1);
    let environment_small_delta = small_delta(environment_uncertainty_sq, c, p2, outcome2);

    // Eta is equivalent to large delta as there are only two teams.
    let players_eta = eta(
        players_uncertainty_sq,
        c,
        p1,
        gamma(players_uncertainty_sq, c),
    );
    let environment_eta = eta(
        environment_uncertainty_sq,
        c,
        p2,
        gamma(environment_uncertainty_sq, c),
    );

    let mut new_players = Vec::new();
    let mut new_environment = Vec::new();

    for player in players_team {
        let player_uncertainty_squared = player.uncertainty.powi(2);
        let new_rating = new_rating_teams(
            player.rating + player.loadout_modifier,
            player_uncertainty_squared,
            players_uncertainty_sq,
            players_small_delta,
        ) - player.loadout_modifier;
        let new_uncertainty = new_uncertainty_teams(
            player_uncertainty_squared,
            players_uncertainty_sq,
            config.uncertainty_tolerance,
            players_eta,
        );

        new_players.push(MhthRating {
            rating: new_rating,
            loadout_modifier: player.loadout_modifier,
            uncertainty: new_uncertainty,
        });
    }

    for env in environment {
        let env_uncertainty_sq = env.uncertainty.powi(2);
        let new_rating = new_rating_teams(
            env.rating + env.loadout_modifier,
            env_uncertainty_sq,
            environment_uncertainty_sq,
            environment_small_delta,
        ) - env.loadout_modifier;
        let new_uncertainty = new_uncertainty_teams(
            env_uncertainty_sq,
            environment_uncertainty_sq,
            config.uncertainty_tolerance,
            environment_eta,
        );

        new_environment.push(MhthRating {
            rating: new_rating,
            loadout_modifier: env.loadout_modifier,
            uncertainty: new_uncertainty,
        });
    }

    (new_players, new_environment)
}

#[must_use]
/// Calculates the [`MhthRating`] of several teams based on their ratings, uncertainties, and ranks of the teams.
///
///
/// Takes in a slice, which contains tuples of teams, which are just slices of [`MhthRating`]s,
/// as well the rank of the team as an [`MultiTeamOutcome`] and a [`MhthConfig`].
///
/// Ties are represented by several teams having the same rank.
///
/// Returns new ratings and uncertainties of players in the teams in the same order.
///
/// Similar to [`mhth_team_vs_environment`].
///
/// > Good for player teams vs multiple environment missions acting together.
/// > Or multiple player teams vs single or multiple environment missions.
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::{
///     MultiTeamOutcome,
///     mhth::{MhthConfig, MhthRating, mhth_multi_team},
/// };
///
/// let players_team = vec![
///     MhthRating::new(),
///     MhthRating {
///         rating: 30.0,
///         loadout_modifier: 3.0,
///         uncertainty: 1.2,
///     },
///     MhthRating {
///         rating: 21.0,
///         loadout_modifier: 3.3,
///         uncertainty: 6.5,
///     },
/// ];
///
/// let environment_team_1 = vec![
///     MhthRating::default(),
///     MhthRating {
///         rating: 41.0,
///         loadout_modifier: 1.0,
///         uncertainty: 1.4,
///     },
///     MhthRating {
///         rating: 19.2,
///         loadout_modifier: 1.2,
///         uncertainty: 4.3,
///     },
/// ];
///
/// let environment_team_2 = vec![
///     MhthRating::default(),
///     MhthRating {
///         rating: 29.4,
///         loadout_modifier: 1.1,
///         uncertainty: 1.6,
///     },
///     MhthRating {
///         rating: 17.2,
///         loadout_modifier: 1.2,
///         uncertainty: 2.1,
///     },
/// ];
///
/// let teams_and_ranks = vec![
///     (&players_team[..], MultiTeamOutcome::new(2)), // Team 1 takes the second place.
///     (&environment_team_1[..], MultiTeamOutcome::new(1)), // Team 2 takes the first place.
///     (&environment_team_2[..], MultiTeamOutcome::new(3)), // Team 3 takes the third place.
/// ];
///
/// let new_teams = mhth_multi_team(&teams_and_ranks, &MhthConfig::new());
///
/// assert_eq!(new_teams.len(), 3);
///
/// let new_players_team = &new_teams[0];
/// let new_environment_team_1 = &new_teams[1];
/// let new_environment_team_2 = &new_teams[2];
///
/// assert_eq_float!((new_players_team[0].rating * 100.0).round(), 2480.0); // 2538.0 for openskill
/// assert_eq_float!((new_players_team[1].rating * 100.0).round(), 3000.0); // 3001.0 for openskill
/// assert_eq_float!((new_players_team[2].rating * 100.0).round(), 2088.0); // 2123.0 for openskill
///
/// assert_eq_float!((new_environment_team_1[0].rating * 100.0).round(), 2825.0); // 2796.0 for openskill
/// assert_eq_float!((new_environment_team_1[1].rating * 100.0).round(), 4109.0); // 4108.0 for openskill
/// assert_eq_float!((new_environment_team_1[2].rating * 100.0).round(), 2006.0); // 1999.0 for openskill
///
/// assert_eq_float!((new_environment_team_2[0].rating * 100.0).round(), 2195.0); // 2166.0 for openskill
/// assert_eq_float!((new_environment_team_2[1].rating * 100.0).round(), 2929.0); // 2928.0 for openskill
/// assert_eq_float!((new_environment_team_2[2].rating * 100.0).round(), 1701.0); // 1699.0 for openskill
/// ```
pub fn mhth_multi_team(
    teams_and_ranks: &[(&[MhthRating], MultiTeamOutcome)],
    config: &MhthConfig,
) -> Vec<Vec<MhthRating>> {
    if teams_and_ranks.is_empty() {
        return Vec::new();
    }

    // Just returning the original teams if a team is empty.
    for (team, _) in teams_and_ranks {
        if team.is_empty() {
            return teams_and_ranks
                .iter()
                .map(|(team, _)| team.to_vec())
                .collect();
        }
    }

    let mut teams_ratings = Vec::with_capacity(teams_and_ranks.len());
    let mut teams_uncertainties_sq = Vec::with_capacity(teams_and_ranks.len());

    for (team, _) in teams_and_ranks {
        let team_rating: f64 = team.iter().map(|p| p.rating + p.loadout_modifier).sum();
        let team_uncertainty_sq: f64 = team.iter().map(|p| p.uncertainty.powi(2)).sum();

        teams_ratings.push(team_rating);
        teams_uncertainties_sq.push(team_uncertainty_sq);
    }

    let mut new_teams = Vec::with_capacity(teams_and_ranks.len());
    for (i, (team_one, rank_one)) in teams_and_ranks.iter().enumerate() {
        let mut omega = 0.0;
        let mut large_delta = 0.0;

        for (q, (_, rank_two)) in teams_and_ranks.iter().enumerate() {
            if i == q {
                continue;
            }

            let c = 2.0f64
                .mul_add(
                    config.beta.powi(2),
                    teams_uncertainties_sq[i] + teams_uncertainties_sq[q],
                )
                .sqrt();

            let (p, _) = p_value(teams_ratings[i], teams_ratings[q], c);
            let score = match rank_two.cmp(rank_one) {
                Ordering::Greater => 1.0,
                Ordering::Equal => 0.5,
                Ordering::Less => 0.0,
            };

            let small_delta = small_delta(teams_uncertainties_sq[i], c, p, score);
            let eta = eta(
                teams_uncertainties_sq[i],
                c,
                p,
                gamma(teams_uncertainties_sq[i], c),
            );

            omega += small_delta;
            large_delta += eta;
        }

        let mut new_team = Vec::with_capacity(team_one.len());
        for player in *team_one {
            let player_uncertainty_sq = player.uncertainty.powi(2);
            let new_rating = new_rating_teams(
                player.rating + player.loadout_modifier,
                player_uncertainty_sq,
                teams_uncertainties_sq[i],
                omega,
            ) - player.loadout_modifier;
            let new_uncertainty = new_uncertainty_teams(
                player_uncertainty_sq,
                teams_uncertainties_sq[i],
                config.uncertainty_tolerance,
                large_delta,
            );

            new_team.push(MhthRating {
                rating: new_rating,
                loadout_modifier: player.loadout_modifier,
                uncertainty: new_uncertainty,
            });
        }
        new_teams.push(new_team);
    }

    new_teams
}

#[must_use]
/// Calculates the expected outcome of two players based on the Bradley-Terry model.
///
/// Takes in two players as [`MhthRating`]s and a [`MhthConfig`],
/// and returns the probability of victory for each player as an [`f64`] between 1.0 and 0.0.
///
/// 1.0 means a certain victory for the player, 0.0 means certain loss.
/// Values near 0.5 mean a draw is likely to occur.
///
/// Similar to [`expected_team_vs_environment`] and [`expected_score_multi_team`].
///
/// > Expected score for single player match, based on [`mhth`]
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::mhth::{MhthConfig, MhthRating, expected_score};
///
/// let player = MhthRating {
///     rating: 42.0,
///     loadout_modifier: 5.0,
///     uncertainty: 2.1,
/// };
/// let environment = MhthRating {
///     rating: 31.0,
///     loadout_modifier: 0.0,
///     uncertainty: 1.2,
/// };
///
/// let (exp1, exp2) = expected_score(&player, &environment, &MhthConfig::new());
///
/// assert_eq_float!(exp1 + exp2, 1.0);
///
/// assert_eq_float!((exp1 * 100.0).round(), 92.0); // 85.0 for openskill
/// ```
pub fn expected_score(
    player: &MhthRating,
    environment: &MhthRating,
    config: &MhthConfig,
) -> (f64, f64) {
    let c = 2.0f64
        .mul_add(
            config.beta.powi(2),
            player
                .uncertainty
                .mul_add(player.uncertainty, environment.uncertainty.powi(2)),
        )
        .sqrt();

    p_value(
        player.rating + player.loadout_modifier,
        environment.rating + environment.loadout_modifier,
        c,
    )
}

#[must_use]
/// Calculates the expected outcome of two teams based on the Bradley-Terry model.
///
/// Takes in two teams as a Slice of [`MhthRating`]s and a [`MhthConfig`],
/// and returns the probability of victory for each player as an [`f64`] between 1.0 and 0.0.
///
/// 1.0 means a certain victory for the player, 0.0 means certain loss.
/// Values near 0.5 mean a draw is likely to occur.
///
/// Similar to [`expected_score`] and [`expected_score_multi_team`].
///
/// > Expected score for team vs environment, following [`mhth_team_vs_environment`] rules.
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::mhth::{MhthConfig, MhthRating, expected_team_vs_environment};
///
/// let players_team = vec![
///     MhthRating {
///         rating: 42.0,
///         loadout_modifier: 5.0,
///         uncertainty: 2.1,
///     },
///     MhthRating::new(),
///     MhthRating {
///         rating: 12.0,
///         loadout_modifier: 2.0,
///         uncertainty: 3.2,
///     },
/// ];
/// let environment = vec![
///     MhthRating {
///         rating: 31.0,
///         loadout_modifier: 0.0,
///         uncertainty: 1.2,
///     },
///     MhthRating::new(),
///     MhthRating {
///         rating: 41.0,
///         loadout_modifier: 0.0,
///         uncertainty: 1.2,
///     },
/// ];
///
/// let (exp1, exp2) =
///     expected_team_vs_environment(&players_team, &environment, &MhthConfig::new());
///
/// assert_eq_float!(exp1 + exp2, 1.0);
///
/// assert_eq_float!((exp1 * 100.0).round(), 31.0); // 21.0 for openskill
/// ```
pub fn expected_team_vs_environment(
    players_team: &[MhthRating],
    environment: &[MhthRating],
    config: &MhthConfig,
) -> (f64, f64) {
    let players_team_rating: f64 = players_team
        .iter()
        .map(|p| p.rating + p.loadout_modifier)
        .sum();
    let environment_rating: f64 = environment
        .iter()
        .map(|p| p.rating + p.loadout_modifier)
        .sum();

    let players_team_uncertainty_sq: f64 = players_team.iter().map(|p| p.uncertainty.powi(2)).sum();
    let environment_uncertainty_sq: f64 = environment.iter().map(|p| p.uncertainty.powi(2)).sum();

    let c = 2.0f64
        .mul_add(
            config.beta.powi(2),
            players_team_uncertainty_sq + environment_uncertainty_sq,
        )
        .sqrt();

    p_value(players_team_rating, environment_rating, c)
}

#[must_use]
/// Calculates the expected outcome of mulitple teams based on the Bradley-Terry model.
///
/// Takes in a slice of teams as a slice of [`MhthRating`]s and a [`MhthConfig`],
/// and returns the probability of victory for each team as an [`f64`] between 1.0 and 0.0.
///
/// 1.0 means a certain victory for the team, 0.0 means certain loss.
/// Values near `1 / Number of Teams` mean a draw is likely to occur.
///
/// Similar to [`expected_score`] and [`expected_team_vs_environment`].
///
/// > Expected score for player teams vs multiple environment missions acting together.
/// > Or multiple player teams vs single or multiple environment missions.
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::mhth::{MhthConfig, MhthRating, expected_score_multi_team};
///
/// let players_team = vec![
///     MhthRating {
///         rating: 42.0,
///         loadout_modifier: 5.0,
///         uncertainty: 2.1,
///     },
///     MhthRating::new(),
///     MhthRating {
///         rating: 12.0,
///         loadout_modifier: 2.0,
///         uncertainty: 3.2,
///     },
/// ];
/// let environment_1 = vec![
///     MhthRating {
///         rating: 31.0,
///         loadout_modifier: 0.0,
///         uncertainty: 1.2,
///     },
///     MhthRating::new(),
///     MhthRating {
///         rating: 41.0,
///         loadout_modifier: 0.0,
///         uncertainty: 1.2,
///     },
/// ];
/// let environment_2 = vec![
///     MhthRating {
///         rating: 31.0,
///         loadout_modifier: 1.2,
///         uncertainty: 1.2,
///     },
///     MhthRating::new(),
///     MhthRating {
///         rating: 41.0,
///         loadout_modifier: 0.0,
///         uncertainty: 1.2,
///     },
/// ];
///
/// let exp = expected_score_multi_team(
///     &[&players_team, &environment_1, &environment_2],
///     &MhthConfig::new(),
/// );
///
/// assert_eq_float!(exp[0] + exp[1] + exp[2], 1.0);
/// assert_eq_float!((exp[0] * 100.0).round(), 20.0); // 14.0 for openskill
/// assert_eq_float!((exp[1] * 100.0).round(), 39.0); // 43.0 for openskill
/// assert_eq_float!((exp[2] * 100.0).round(), 42.0); // 43.0 for openskill
/// ```
pub fn expected_score_multi_team(teams: &[&[MhthRating]], config: &MhthConfig) -> Vec<f64> {
    let mut ratings = Vec::with_capacity(teams.len());

    for team in teams {
        let team_rating: f64 = team.iter().map(|p| p.rating + p.loadout_modifier).sum();
        ratings.push(team_rating);
    }

    let mut uncertainties_sq = Vec::with_capacity(teams.len());

    for team in teams {
        let team_uncertainty_sq: f64 = team.iter().map(|p| p.uncertainty.powi(2)).sum();
        uncertainties_sq.push(team_uncertainty_sq);
    }

    let c = 2.0f64
        .mul_add(config.beta.powi(2), uncertainties_sq.iter().sum::<f64>())
        .sqrt();

    let mut exps = Vec::with_capacity(ratings.len());

    let mut sum = 0.0;

    for rating in ratings {
        let e = (rating / c).exp();
        exps.push(e);
        sum += e;
    }

    for exp in &mut exps {
        *exp /= sum;
    }

    exps
}

#[must_use]
/// Calculates the expected outcome of a player in a rating period or tournament.
///
/// Takes in a player as [`MhthRating`], a list of environment missions as a slice of [`MhthRating`] and a [`MhthConfig`]
/// and returns the probability of victory for each match as an Vec of [`f64`] between 1.0 and 0.0 from the perspective of the player.
/// 1.0 means a certain victory for the player, 0.0 means certain loss.
/// Values near 0.5 mean a draw is likely to occur.
///
/// > Expected score for single player vs multiple environment missions, based on [`mhth_rating_period`]
///
/// # Examples
/// ```rust
/// # use assert_eq_float::assert_eq_float;
/// use skillratings::mhth::{MhthConfig, MhthRating, expected_score_rating_period};
///
/// let player = MhthRating {
///     rating: 19.0,
///     loadout_modifier: 5.0,
///     uncertainty: 4.0,
/// };
///
/// let environment_1 = MhthRating {
///     rating: 19.3,
///     loadout_modifier: 1.0,
///     uncertainty: 4.0,
/// };
///
/// let environment_2 = MhthRating {
///     rating: 17.3,
///     loadout_modifier: 1.0,
///     uncertainty: 4.0,
/// };
///
/// let config = MhthConfig::new();
///
/// let exp = expected_score_rating_period(&player, &[environment_1, environment_2], &config);
///
/// assert_eq_float!((exp[0] * 100.0).round(), 61.0); // 49.0 for openskill
/// assert_eq_float!((exp[1] * 100.0).round(), 67.0); // 55.0 for openskill
/// ```
pub fn expected_score_rating_period(
    player: &MhthRating,
    opponents: &[MhthRating],
    config: &MhthConfig,
) -> Vec<f64> {
    opponents
        .iter()
        .map(|o| expected_score(player, o, config).0)
        .collect()
}

fn p_value(rating_one: f64, rating_two: f64, c_value: f64) -> (f64, f64) {
    let e1 = (rating_one / c_value).exp();
    let e2 = (rating_two / c_value).exp();

    let exp_one = e1 / (e1 + e2);
    let exp_two = 1.0 - exp_one;

    (exp_one, exp_two)
}

fn small_delta(team_uncertainty_sq: f64, c_value: f64, p_value: f64, score: f64) -> f64 {
    (team_uncertainty_sq / c_value) * (score - p_value)
}

// You could also set gamma to 1/k, with k being the amount of teams in a match.
// But you need to change the 1v1 uncertainty function below accordingly.
fn gamma(team_uncertainty_sq: f64, c_value: f64) -> f64 {
    team_uncertainty_sq.sqrt() / c_value
}

fn eta(team_uncertainty_sq: f64, c_value: f64, p_value: f64, gamma: f64) -> f64 {
    gamma * team_uncertainty_sq / c_value.powi(2) * p_value * (1.0 - p_value)
}

// We separate the 1v1 and teams functions, because we can use a few shortcuts on the 1v1 functions to increase performance.
fn new_rating(
    player_rating: f64,
    player_uncertainty: f64,
    c_value: f64,
    p_value: f64,
    score: f64,
) -> f64 {
    (player_uncertainty.powi(2) / c_value).mul_add(score - p_value, player_rating)
}

fn new_uncertainty(
    player_uncertainty: f64,
    c_value: f64,
    p_value: f64,
    uncertainty_tolerance: f64,
) -> f64 {
    let eta = (player_uncertainty / c_value).powi(3) * p_value * (1.0 - p_value);
    (player_uncertainty.powi(2) * (1.0 - eta).max(uncertainty_tolerance)).sqrt()
}

fn new_rating_teams(
    player_rating: f64,
    player_uncertainty_sq: f64,
    team_uncertainty_sq: f64,
    omega: f64,
) -> f64 {
    (player_uncertainty_sq / team_uncertainty_sq).mul_add(omega, player_rating)
}

fn new_uncertainty_teams(
    player_uncertainty_sq: f64,
    team_uncertainty_sq: f64,
    uncertainty_tolerance: f64,
    large_delta: f64,
) -> f64 {
    let new_player_uncertainty_sq = (player_uncertainty_sq / team_uncertainty_sq)
        .mul_add(-large_delta, 1.0)
        .max(uncertainty_tolerance);
    (player_uncertainty_sq * new_player_uncertainty_sq).sqrt()
}
#[cfg(test)]
mod tests {
    use super::*;
    use assert_eq_float::assert_eq_float;

    #[test]
    fn test_progression() {
        let players_team = vec![
            MhthRating {
                rating: 319.0,
                loadout_modifier: 21.0,
                uncertainty: 4.0,
            },
            MhthRating {
                rating: 289.0,
                loadout_modifier: 28.0,
                uncertainty: 5.7,
            },
            MhthRating {
                rating: 297.0,
                loadout_modifier: 18.0,
                uncertainty: 7.0,
            },
        ];

        let environment_hard_boss = vec![MhthRating {
            rating: 1012.0,
            loadout_modifier: 3.0,
            uncertainty: 12.0,
        }];

        let config = MhthConfig::default();
        let (player_win_chance, boss_win_chance) =
            expected_team_vs_environment(&players_team, &environment_hard_boss, &config);

        assert!(player_win_chance < boss_win_chance);
        assert_eq_float!((player_win_chance * 10000.0).round() / 100., 7.); // 7%
        assert_eq_float!((boss_win_chance * 10000.0).round() / 100., 93.); // 93%

        // assuming players won against super boss:
        let (players_updated_ratings, _) = mhth_team_vs_environment(
            &players_team,
            &environment_hard_boss,
            &Outcomes::SUCCESSFUL,
            &config,
        );

        // Players should have ratings increased
        assert_eq_float!(players_updated_ratings[0].rating.round(), 320.0);
        assert_eq_float!(players_updated_ratings[1].rating.round(), 291.0);
        assert_eq_float!(players_updated_ratings[2].rating.round(), 300.0);

        // assuming players won against easy boss (no actual gain):
        let environment_easy_boss = vec![MhthRating {
            rating: 280.0,
            loadout_modifier: 3.0,
            uncertainty: 12.0,
        }];
        let (players_updated_ratings, _) = mhth_team_vs_environment(
            &players_team,
            &environment_easy_boss,
            &Outcomes::SUCCESSFUL,
            &config,
        );

        // Players should have ratings increased
        assert_eq_float!(players_updated_ratings[0].rating.round(), 319.0); // no actual gain
        assert_eq_float!(players_updated_ratings[1].rating.round(), 289.0); // no actual gain
        assert_eq_float!(players_updated_ratings[2].rating.round(), 297.0); // no actual gain

        // assuming players lost against easy boss with multiple bots (ratins increase):
        let mut environment_easy_boss_with_bots = vec![MhthRating {
            rating: 280.0,
            loadout_modifier: 3.0,
            uncertainty: 12.0,
        }];
        let mut bots = (0..50)
            .map(|_| MhthRating {
                rating: 53.0,
                loadout_modifier: 12.0,
                uncertainty: 2.0,
            })
            .collect::<Vec<_>>();
        environment_easy_boss_with_bots.append(&mut bots);
        let (players_updated_ratings, _) = mhth_team_vs_environment(
            &players_team,
            &environment_easy_boss_with_bots,
            &Outcomes::SUCCESSFUL,
            &config,
        );

        // Players should have ratings increased
        assert_eq_float!(players_updated_ratings[0].rating.round(), 320.0);
        assert_eq_float!(players_updated_ratings[1].rating.round(), 290.0);
        assert_eq_float!(players_updated_ratings[2].rating.round(), 299.0);
    }
}
