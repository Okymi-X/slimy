use slimy_core::{check, headline, DayLog, Gene, Genome, Soul};

fn soul() -> Soul {
    Soul::new("Okymi", "Mochi", "fr", 1_000).unwrap()
}

fn busy_day(day: i64) -> DayLog {
    DayLog {
        day,
        caress_seconds: 300.0,
        throws: 4,
        answered: 2,
        ignored: 0,
        messages: 5,
        active_seconds: 7_200.0,
        night_seconds: 0.0,
        calmed_alone: 0,
    }
}

fn in_range(v: f32) -> bool {
    (-1.0..=1.0).contains(&v)
}

#[test]
fn a_quiet_day_gives_no_signal() {
    let s = DayLog::started(5).signals();
    assert_eq!(
        [s.affection, s.energy, s.curiosity, s.night_use, s.tenacity],
        [0.0; 5]
    );
}

#[test]
fn caresses_and_answers_raise_affection_and_ignored_questions_lower_it() {
    let warm = DayLog {
        caress_seconds: 240.0,
        answered: 2,
        ..DayLog::started(1)
    }
    .signals();
    let cold = DayLog {
        ignored: 4,
        ..DayLog::started(1)
    }
    .signals();
    assert!(warm.affection > 0.8);
    assert!(cold.affection < -0.5);
}

#[test]
fn any_day_however_hostile_gives_signals_in_range() {
    let hostile = DayLog {
        day: 3,
        caress_seconds: f32::INFINITY,
        throws: u32::MAX,
        answered: u32::MAX,
        ignored: 0,
        messages: u32::MAX,
        active_seconds: f32::NAN,
        night_seconds: -50.0,
        calmed_alone: u32::MAX,
    };
    let mut log = DayLog::started(3);
    for _ in 0..1_000 {
        log.add(&hostile);
    }
    let s = log.signals();
    for v in [s.affection, s.energy, s.curiosity, s.night_use, s.tenacity] {
        assert!(in_range(v) && !v.is_nan(), "{s:?}");
    }
    assert!(log.caress_seconds <= 86_400.0 && log.throws <= 10_000);
}

#[test]
fn one_report_cannot_claim_more_than_a_plausible_amount() {
    let mut log = DayLog::started(1);
    log.add(&DayLog {
        caress_seconds: 1.0e9,
        throws: 1_000_000,
        ..DayLog::default()
    });
    assert_eq!(log.caress_seconds, 3_600.0);
    assert_eq!(log.throws, 200);
}

#[test]
fn the_first_call_only_opens_the_first_day() {
    let mut soul = soul();
    assert!(soul.settle_day(20_000).is_none());
    assert_eq!(soul.day.day, 20_000);
    assert_eq!(soul.genome.nights, 0);
}

#[test]
fn the_same_day_digests_nothing() {
    let mut soul = soul();
    soul.settle_day(20_000);
    assert!(soul.record_day(20_000, &busy_day(0)).is_none());
    assert_eq!(soul.genome.nights, 0);
    assert_eq!(soul.day.throws, 4);
}

#[test]
fn a_new_day_turns_the_old_one_into_one_night() {
    let mut soul = soul();
    soul.settle_day(20_000);
    soul.record_day(20_000, &busy_day(0));
    let before = soul.genome.clone();
    let night = soul
        .record_day(20_001, &DayLog::default())
        .expect("a night");
    assert_eq!(night.night, 1);
    assert_eq!(soul.genome.nights, 1);
    assert_eq!(soul.day.day, 20_001);
    assert_eq!(soul.day.throws, 0, "the new day starts empty");
    assert!(
        soul.genome.farthest_gene(&before) > 0.0,
        "a busy day changes the genome"
    );
    assert!(check(&soul.genome).is_good());
}

#[test]
fn a_gap_of_many_days_is_one_night_not_many() {
    let mut soul = soul();
    soul.settle_day(20_000);
    soul.record_day(20_000, &busy_day(0));
    soul.settle_day(20_030);
    assert_eq!(soul.genome.nights, 1);
}

#[test]
fn a_clock_that_goes_backwards_changes_nothing() {
    let mut soul = soul();
    soul.settle_day(20_000);
    soul.record_day(20_000, &busy_day(0));
    let before = soul.clone();
    assert!(soul.settle_day(19_990).is_none());
    assert_eq!(soul, before);
}

#[test]
fn the_same_life_gives_the_same_nights() {
    let live = || {
        let mut soul = soul();
        soul.settle_day(20_000);
        for day in 20_000..20_040 {
            soul.record_day(day, &busy_day(0));
        }
        soul.settle_day(20_040);
        soul.genome
    };
    assert_eq!(live(), live());
}

#[test]
fn a_loving_week_warms_the_cheeks_and_a_lonely_one_does_not() {
    let week = |log: DayLog| {
        let mut soul = soul();
        soul.settle_day(20_000);
        for day in 20_000..20_007 {
            soul.record_day(day, &log);
        }
        soul.settle_day(20_007);
        soul.genome.get(Gene::Cheeks)
    };
    let loved = week(DayLog {
        caress_seconds: 300.0,
        answered: 2,
        ..DayLog::default()
    });
    let neglected = week(DayLog {
        ignored: 4,
        active_seconds: 600.0,
        ..DayLog::default()
    });
    assert!(loved > neglected, "{loved} against {neglected}");
}

#[test]
fn the_journal_talks_about_the_gene_that_moved_most() {
    let mut soul = soul();
    soul.settle_day(20_000);
    soul.record_day(
        20_000,
        &DayLog {
            night_seconds: 7_200.0,
            ..DayLog::default()
        },
    );
    let night = soul.settle_day(20_001).expect("a night");
    let (gene, delta) = headline(&night).expect("something moved");
    assert!(matches!(gene, Gene::Stars | Gene::SkinHue), "{gene:?}");
    assert!(delta != 0.0);
}

#[test]
fn a_soul_with_a_genome_and_a_day_survives_a_round_trip() {
    let mut soul = soul();
    soul.settle_day(20_000);
    soul.record_day(20_000, &busy_day(0));
    soul.record_day(20_001, &DayLog::default());
    let back = Soul::from_json(&soul.to_json()).unwrap();
    assert_eq!(back.genome.nights, 1);
    assert_eq!(back.day.day, 20_001);
    assert!(back.genome.farthest_gene(&soul.genome) < 1e-6);
}

#[test]
fn a_soul_written_before_the_genome_loads_as_the_base_slime() {
    let old = r#"{"version":1,"user_name":"Okymi","slime_name":"Mochi","language":"fr","born_at":42,
        "tone":{"affection":0.5,"curiosity":0.5,"energy":0.5,"tenacity":0.5},"facts":[]}"#;
    let soul = Soul::from_json(old).unwrap();
    assert_eq!(soul.genome, Genome::default());
    assert_eq!(soul.day, DayLog::default());
}

#[test]
fn a_hostile_genome_inside_a_soul_is_sanitized() {
    let mut value: serde_json::Value = serde_json::from_str(&soul().to_json()).unwrap();
    value["genome"]["genes"] = serde_json::json!([50.0, -50.0]);
    value["genome"]["history"] =
        serde_json::json!([{"night": 1, "seed": 1, "deltas": [[99, 1.0]], "mutation": 99}]);
    value["day"]["caress_seconds"] = serde_json::json!(-5.0);
    let soul = Soul::from_json(&value.to_string()).unwrap();
    for gene in Gene::ALL {
        assert!((0.0..=1.0).contains(&soul.genome.get(gene)));
    }
    assert!(soul.genome.history[0].deltas.is_empty());
}

#[test]
fn a_night_report_names_genes_and_never_indices() {
    let mut soul = soul();
    soul.settle_day(20_000);
    soul.record_day(
        20_000,
        &DayLog {
            night_seconds: 7_200.0,
            ..DayLog::default()
        },
    );
    let night = soul.settle_day(20_001).expect("a night");
    let report = slimy_core::report(&night);
    assert_eq!(report.night, 1);
    assert!(
        matches!(report.headline, Some("stars") | Some("skin_hue")),
        "{report:?}"
    );
    assert!(report.change != 0.0);
    let json = serde_json::to_value(&report).unwrap();
    assert!(json["headline"].is_string());
}

#[test]
fn the_same_seed_gives_the_same_hatchling_whatever_the_birth_time() {
    let a = Soul::with_seed("A", "B", "fr", 100, 777).unwrap();
    let b = Soul::with_seed("C", "D", "en", 99_999, 777).unwrap();
    assert_eq!(a.genome, b.genome);
    assert_ne!(
        a.genome,
        Soul::with_seed("A", "B", "fr", 100, 778).unwrap().genome
    );
}
