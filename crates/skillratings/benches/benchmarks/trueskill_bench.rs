use skillratings::{
    MultiTeamOutcome, Outcomes,
    trueskill::{
        TrueSkillConfig, TrueSkillRating, expected_score, expected_score_two_teams, trueskill,
        trueskill_multi_team, trueskill_rating_period, trueskill_two_teams,
    },
};

use criterion::{Criterion, black_box, criterion_group, criterion_main};

pub fn trueskill_benchmark(c: &mut Criterion) {
    let player_one = TrueSkillRating {
        rating: 32.1,
        uncertainty: 4.233,
    };
    let player_two = TrueSkillRating {
        rating: 41.01,
        uncertainty: 1.34,
    };
    let outcome = Outcomes::SUCCESSFUL;
    let config = TrueSkillConfig::new();

    c.bench_function("TrueSkill 1v1", |b| {
        b.iter(|| {
            trueskill(
                black_box(&player_one),
                black_box(&player_two),
                black_box(&outcome),
                black_box(&config),
            )
        })
    });
}

pub fn trueskill_team_benchmark(c: &mut Criterion) {
    let team_one = vec![
        TrueSkillRating {
            rating: 32.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 41.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 32.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 41.01,
            uncertainty: 1.34,
        },
    ];
    let team_two = vec![
        TrueSkillRating {
            rating: 29.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 12.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 9.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 53.01,
            uncertainty: 1.34,
        },
    ];

    let outcome = Outcomes::SUCCESSFUL;
    let config = TrueSkillConfig::new();

    c.bench_function("TrueSkill 4v4", |b| {
        b.iter(|| {
            trueskill_two_teams(
                black_box(&team_one),
                black_box(&team_two),
                black_box(&outcome),
                black_box(&config),
            )
        })
    });
}

pub fn trueskill_multi_team_benchmark(c: &mut Criterion) {
    let team_one = vec![
        TrueSkillRating {
            rating: 32.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 41.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 32.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 41.01,
            uncertainty: 1.34,
        },
    ];
    let team_two = vec![
        TrueSkillRating {
            rating: 29.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 12.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 9.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 53.01,
            uncertainty: 1.34,
        },
    ];
    let team_three = vec![
        TrueSkillRating {
            rating: 29.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 12.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 29.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 53.01,
            uncertainty: 1.34,
        },
    ];
    let team_four = vec![
        TrueSkillRating {
            rating: 29.1,
            uncertainty: 1.233,
        },
        TrueSkillRating {
            rating: 22.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 9.1,
            uncertainty: 6.23,
        },
        TrueSkillRating {
            rating: 13.01,
            uncertainty: 2.34,
        },
    ];

    let config = TrueSkillConfig::new();

    c.bench_function("TrueSkill 4v4v4v4", |b| {
        b.iter(|| {
            trueskill_multi_team(
                &[
                    (&team_one, MultiTeamOutcome::new(1)),
                    (&team_two, MultiTeamOutcome::new(3)),
                    (&team_three, MultiTeamOutcome::new(2)),
                    (&team_four, MultiTeamOutcome::new(2)),
                ],
                black_box(&config),
            )
        })
    });
}

pub fn expected_trueskill(c: &mut Criterion) {
    let player_one = TrueSkillRating {
        rating: 32.1,
        uncertainty: 4.233,
    };
    let player_two = TrueSkillRating {
        rating: 41.01,
        uncertainty: 1.34,
    };

    let config = TrueSkillConfig::new();

    c.bench_function("TrueSkill 1v1 Expected Score", |b| {
        b.iter(|| {
            expected_score(
                black_box(&player_one),
                black_box(&player_two),
                black_box(&config),
            )
        });
    });
}

pub fn expected_trueskill_teams(c: &mut Criterion) {
    let team_one = vec![
        TrueSkillRating {
            rating: 32.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 41.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 32.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 41.01,
            uncertainty: 1.34,
        },
    ];
    let team_two = vec![
        TrueSkillRating {
            rating: 29.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 12.01,
            uncertainty: 1.34,
        },
        TrueSkillRating {
            rating: 9.1,
            uncertainty: 4.233,
        },
        TrueSkillRating {
            rating: 53.01,
            uncertainty: 1.34,
        },
    ];

    let config = TrueSkillConfig::new();

    c.bench_function("TrueSkill 4v4 Expected Score", |b| {
        b.iter(|| {
            expected_score_two_teams(
                black_box(&team_one),
                black_box(&team_two),
                black_box(&config),
            )
        });
    });
}

pub fn rating_period_trueskill(c: &mut Criterion) {
    let player = TrueSkillRating {
        rating: 8.3,
        uncertainty: 2.2,
    };

    let results = vec![
        (
            TrueSkillRating {
                rating: 3.2,
                uncertainty: 2.1,
            },
            Outcomes::SUCCESSFUL,
        ),
        (
            TrueSkillRating {
                rating: 6.2,
                uncertainty: 2.1,
            },
            Outcomes::DRAW,
        ),
        (
            TrueSkillRating {
                rating: 9.2,
                uncertainty: 2.1,
            },
            Outcomes::FAILURE,
        ),
        (
            TrueSkillRating {
                rating: 12.2,
                uncertainty: 2.1,
            },
            Outcomes::SUCCESSFUL,
        ),
        (
            TrueSkillRating {
                rating: 15.2,
                uncertainty: 2.1,
            },
            Outcomes::DRAW,
        ),
        (
            TrueSkillRating {
                rating: 18.2,
                uncertainty: 2.1,
            },
            Outcomes::FAILURE,
        ),
        (
            TrueSkillRating {
                rating: 21.2,
                uncertainty: 2.1,
            },
            Outcomes::SUCCESSFUL,
        ),
        (
            TrueSkillRating {
                rating: 24.2,
                uncertainty: 2.1,
            },
            Outcomes::DRAW,
        ),
        (
            TrueSkillRating {
                rating: 27.2,
                uncertainty: 2.1,
            },
            Outcomes::FAILURE,
        ),
        (
            TrueSkillRating {
                rating: 30.2,
                uncertainty: 2.1,
            },
            Outcomes::FAILURE,
        ),
    ];

    let config = TrueSkillConfig::new();

    c.bench_function("TrueSkill Rating Period 10 Players", |b| {
        b.iter(|| {
            trueskill_rating_period(black_box(&player), black_box(&results), black_box(&config))
        });
    });
}

criterion_group!(
    benches,
    trueskill_benchmark,
    trueskill_team_benchmark,
    trueskill_multi_team_benchmark,
    expected_trueskill,
    expected_trueskill_teams,
    rating_period_trueskill,
);
criterion_main!(benches);
