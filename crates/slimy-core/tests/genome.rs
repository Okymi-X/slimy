use slimy_core::{
    check, gene_distance, harmonize, palette, Gene, Genome, Signals, Stage, MAX_HISTORY,
    MAX_HUE_GAP, MAX_NIGHTLY_STEP, MIN_FACE_CONTRAST, MUTATION_LEVEL,
};

const STEP_TOLERANCE: f32 = 1e-5;

/// A tiny deterministic source of varied days, so the property tests need no extra dependency.
struct Days(u64);

impl Days {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32 / 16_777_216.0) * 2.0 - 1.0
    }

    fn day(&mut self) -> Signals {
        Signals {
            affection: self.next(),
            energy: self.next(),
            curiosity: self.next(),
            night_use: self.next(),
            tenacity: self.next(),
        }
    }
}

fn all_in_range(genome: &Genome) -> bool {
    Gene::ALL
        .iter()
        .all(|g| (0.0..=1.0).contains(&genome.get(*g)))
}

#[test]
fn a_newborn_is_in_range_and_in_harmony() {
    for seed in 0..200 {
        let genome = Genome::newborn(seed);
        assert!(all_in_range(&genome), "seed {seed}");
        assert!(
            check(&genome).is_good(),
            "seed {seed}: {:?}",
            check(&genome)
        );
    }
}

#[test]
fn a_newborn_comes_from_its_seed_alone() {
    assert_eq!(Genome::newborn(7), Genome::newborn(7));
    assert_ne!(Genome::newborn(7), Genome::newborn(8));
}

#[test]
fn a_newborn_has_no_material_yet() {
    let genome = Genome::newborn(3);
    for gene in Gene::MUTABLE {
        assert_eq!(genome.get(gene), 0.0, "{gene:?}");
    }
}

#[test]
fn a_gene_set_out_of_range_is_bounded_and_the_hue_wraps() {
    let mut genome = Genome::default();
    genome.set(Gene::Glow, 5.0);
    assert_eq!(genome.get(Gene::Glow), 1.0);
    genome.set(Gene::Glow, -2.0);
    assert_eq!(genome.get(Gene::Glow), 0.0);
    genome.set(Gene::Glow, f32::NAN);
    assert!((0.0..=1.0).contains(&genome.get(Gene::Glow)));
    genome.set(Gene::SkinHue, 1.25);
    assert!((genome.get(Gene::SkinHue) - 0.25).abs() < 1e-6);
    genome.set(Gene::SkinHue, -0.25);
    assert!((genome.get(Gene::SkinHue) - 0.75).abs() < 1e-6);
}

#[test]
fn life_stages_follow_the_number_of_nights() {
    assert_eq!(Stage::from_nights(0), Stage::Baby);
    assert_eq!(Stage::from_nights(2), Stage::Baby);
    assert_eq!(Stage::from_nights(3), Stage::Child);
    assert_eq!(Stage::from_nights(10), Stage::Teen);
    assert_eq!(Stage::from_nights(25), Stage::Adult);
    assert_eq!(Stage::from_nights(100), Stage::Elder);
}

#[test]
fn the_young_change_faster_than_the_old_but_never_become_another_creature() {
    assert!(Stage::Baby.plasticity() > Stage::Adult.plasticity());
    assert!(Stage::Adult.plasticity() > Stage::Elder.plasticity());
    assert!(Stage::Baby.size_factor() < Stage::Adult.size_factor());
    assert!(Stage::Baby.size_factor() >= 0.5);
}

#[test]
fn one_night_moves_a_gene_by_at_most_the_cap() {
    let mut genome = Genome::newborn(1);
    let before = genome.clone();
    let all_in = Signals {
        affection: 1.0,
        energy: 1.0,
        curiosity: 1.0,
        night_use: 1.0,
        tenacity: 1.0,
    };
    genome.digest(&all_in, 99);
    for gene in Gene::ALL {
        let moved = gene_distance(gene, before.get(gene), genome.get(gene));
        let mutated = genome.history[0].mutation == Some(gene.index() as u8);
        assert!(
            mutated || moved <= MAX_NIGHTLY_STEP + STEP_TOLERANCE,
            "{gene:?} moved {moved}"
        );
    }
}

#[test]
fn a_quiet_day_changes_nothing_but_the_night_counter() {
    let mut genome = Genome::newborn(2);
    let before = genome.clone();
    let record = genome.digest(&Signals::default(), 12_345).clone();
    if record.mutation.is_none() {
        assert!(record.deltas.is_empty());
        assert_eq!(genome.farthest_gene(&before), 0.0);
    }
    assert_eq!(genome.nights, 1);
}

#[test]
fn affection_warms_the_skin_by_the_short_way_round() {
    let mut genome = Genome::default();
    genome.set(Gene::SkinHue, 0.95);
    let day = Signals {
        affection: 1.0,
        ..Signals::default()
    };
    genome.digest(&day, 5);
    // 0.95 is closer to 0.08 going up through 1.0 than going down.
    let hue = genome.get(Gene::SkinHue);
    assert!(hue > 0.95 || hue < 0.08, "hue went the long way: {hue}");
}

#[test]
fn the_same_day_and_seed_give_the_same_night() {
    let day = Signals {
        affection: 0.4,
        energy: -0.2,
        curiosity: 0.9,
        night_use: 0.1,
        tenacity: 0.0,
    };
    let (mut a, mut b) = (Genome::newborn(4), Genome::newborn(4));
    a.digest(&day, 77);
    b.digest(&day, 77);
    assert_eq!(a, b);
}

#[test]
fn a_baby_moves_farther_than_an_elder_for_the_same_day() {
    let day = Signals {
        energy: 1.0,
        curiosity: 1.0,
        ..Signals::default()
    };
    let mut baby = Genome::newborn(6);
    let mut elder = Genome::newborn(6);
    elder.nights = 150;
    let (b0, e0) = (baby.clone(), elder.clone());
    baby.digest(&day, 1);
    elder.digest(&day, 1);
    // Same seed gives the same mutation roll, so compare only unmutated genes.
    let moved = |after: &Genome, before: &Genome| {
        Gene::ALL
            .iter()
            .filter(|g| !Gene::MUTABLE.contains(g))
            .map(|g| gene_distance(*g, before.get(*g), after.get(*g)))
            .sum::<f32>()
    };
    assert!(moved(&baby, &b0) > moved(&elder, &e0));
}

#[test]
fn property_no_sequence_of_nights_leaves_the_range_or_exceeds_the_step() {
    for life in 0..12u64 {
        let mut days = Days(0xABCD_0000 + life);
        let mut genome = Genome::newborn(life);
        for night in 0..300u64 {
            let mut day = days.day();
            if night % 17 == 0 {
                day = Signals {
                    affection: f32::NAN,
                    energy: 9.0,
                    curiosity: -9.0,
                    night_use: f32::INFINITY,
                    tenacity: f32::NEG_INFINITY,
                };
            }
            let before = genome.clone();
            let record = genome.digest(&day, life * 1000 + night).clone();
            assert!(all_in_range(&genome), "life {life} night {night}");
            for gene in Gene::ALL {
                let mutated = record.mutation == Some(gene.index() as u8);
                let moved = gene_distance(gene, before.get(gene), genome.get(gene));
                assert!(
                    mutated || moved <= MAX_NIGHTLY_STEP + STEP_TOLERANCE,
                    "life {life} night {night}: {gene:?} moved {moved}"
                );
            }
            assert!(
                check(&genome).is_good(),
                "life {life} night {night}: {:?}",
                check(&genome)
            );
        }
        assert!(genome.history.len() <= MAX_HISTORY);
    }
}

#[test]
fn property_a_mutation_only_switches_on_something_absent_and_never_removes() {
    let mut mutations = 0;
    for life in 0..40u64 {
        let mut genome = Genome::newborn(life);
        for night in 0..200u64 {
            let before = genome.clone();
            let record = genome
                .digest(&Signals::default(), life * 10_000 + night)
                .clone();
            if let Some(index) = record.mutation {
                mutations += 1;
                let gene = Gene::from_index(usize::from(index)).unwrap();
                assert!(Gene::MUTABLE.contains(&gene));
                assert!(before.get(gene) < MUTATION_LEVEL);
                assert_eq!(genome.get(gene), MUTATION_LEVEL);
            }
            for gene in Gene::MUTABLE {
                assert!(
                    genome.get(gene) >= before.get(gene) - STEP_TOLERANCE,
                    "{gene:?} shrank in a quiet night"
                );
            }
        }
    }
    assert!(
        mutations > 50,
        "only {mutations} mutations in 8000 quiet nights, expected about 240"
    );
}

#[test]
fn replaying_the_history_rebuilds_the_genome() {
    let start = Genome::newborn(11);
    let mut genome = start.clone();
    let mut days = Days(42);
    for night in 0..60u64 {
        genome.digest(&days.day(), night);
    }
    let replayed = Genome::replayed(&start, &genome.history);
    assert_eq!(replayed.nights, genome.nights);
    assert!(
        replayed.farthest_gene(&genome) < 1e-3,
        "replay drifted by {}",
        replayed.farthest_gene(&genome)
    );
}

#[test]
fn the_history_is_capped() {
    let mut genome = Genome::newborn(1);
    for night in 0..(MAX_HISTORY as u64 + 50) {
        genome.digest(&Signals::default(), night);
    }
    assert_eq!(genome.history.len(), MAX_HISTORY);
    assert_eq!(genome.nights as usize, MAX_HISTORY + 50);
    assert_eq!(
        genome.history.last().unwrap().night as usize,
        MAX_HISTORY + 50
    );
}

#[test]
fn a_wide_hue_gap_is_brought_back_inside_the_rule() {
    let mut genome = Genome::newborn(1);
    genome.set(Gene::CoreShift, 1.0);
    assert!(check(&genome).hue_gap > MAX_HUE_GAP);
    assert!(harmonize(&mut genome));
    assert!(
        check(&genome).hue_gap <= MAX_HUE_GAP + 1e-3,
        "{:?}",
        check(&genome)
    );
}

#[test]
fn a_face_lost_in_a_dark_skin_is_brought_back_to_readable() {
    let mut genome = Genome::newborn(1);
    genome.set(Gene::Lightness, 0.0);
    assert!(check(&genome).face_contrast < MIN_FACE_CONTRAST);
    assert!(harmonize(&mut genome));
    assert!(check(&genome).face_contrast >= MIN_FACE_CONTRAST);
}

#[test]
fn a_genome_already_in_harmony_is_left_alone() {
    let mut genome = Genome::newborn(9);
    let before = genome.clone();
    assert!(!harmonize(&mut genome));
    assert_eq!(genome, before);
}

#[test]
fn the_palette_is_valid_colour_for_any_hue_and_saturation() {
    for hue in 0..=20 {
        for saturation in [0.0, 0.5, 1.0] {
            let mut genome = Genome::newborn(0);
            genome.set(Gene::SkinHue, hue as f32 / 20.0);
            genome.set(Gene::Saturation, saturation);
            let p = palette(&genome);
            for colour in [p.top, p.mid, p.bot, p.light, p.core, p.face] {
                assert!(
                    colour.iter().all(|c| (0.0..=1.0).contains(c)),
                    "hue {hue} sat {saturation}: {colour:?}"
                );
            }
            assert!(
                p.top.iter().sum::<f32>() > p.bot.iter().sum::<f32>(),
                "the top of the gel is lighter than its base"
            );
        }
    }
}

#[test]
fn the_base_slime_is_blue() {
    let p = palette(&Genome::default());
    assert!(p.mid[2] > p.mid[0] && p.mid[2] > p.mid[1], "{:?}", p.mid);
}

#[test]
fn a_stored_genome_is_sanitized_when_loaded() {
    let mut genome = Genome::newborn(5);
    genome.digest(
        &Signals {
            affection: 1.0,
            ..Signals::default()
        },
        1,
    );
    let json = genome.to_json();
    let back = Genome::from_json(&json).expect("round trip");
    assert!(back.farthest_gene(&genome) < 1e-6);
    assert_eq!(back.nights, genome.nights);

    let hostile = r#"{"genes":[9.0,-4.0,0.5],"nights":3,"history":[{"night":1,"seed":1,"deltas":[[200,0.5],[0,7.0]],"mutation":250}]}"#;
    let loaded = Genome::from_json(hostile).expect("hostile but well formed");
    assert!(all_in_range(&loaded));
    assert_eq!(
        loaded.history[0].deltas.len(),
        1,
        "an unknown gene index is dropped"
    );
    assert_eq!(loaded.history[0].mutation, None);
    assert!(loaded.history[0].deltas[0].1 <= 1.0);
    assert!(check(&loaded).is_good());

    assert!(Genome::from_json("not json").is_none());
}

#[test]
fn a_stored_history_cannot_grow_without_bound() {
    let mut genome = Genome::newborn(1);
    genome.digest(&Signals::default(), 1);
    let record = genome.history[0].clone();
    let json = serde_json::json!({ "genes": [0.5], "nights": 0, "history": vec![record; MAX_HISTORY * 3] }).to_string();
    let loaded = Genome::from_json(&json).unwrap();
    assert_eq!(loaded.history.len(), MAX_HISTORY);
}

fn base_model_colors() -> [[f32; 3]; 5] {
    let json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../apps/desktop/web/models/base-slime.json"
    ))
    .unwrap();
    let colors = &json["material"]["colors"];
    let read = |name: &str| {
        let v = &colors[name];
        [
            v[0].as_f64().unwrap() as f32,
            v[1].as_f64().unwrap() as f32,
            v[2].as_f64().unwrap() as f32,
        ]
    };
    [
        read("top"),
        read("mid"),
        read("bot"),
        read("light"),
        read("core"),
    ]
}

#[test]
fn the_default_genome_looks_like_the_base_slime_model() {
    let look = Genome::default().look();
    let base = base_model_colors();
    let made = [
        look.colors.top,
        look.colors.mid,
        look.colors.bot,
        look.colors.light,
        look.colors.core,
    ];
    for (layer, (a, b)) in made.iter().zip(base.iter()).enumerate() {
        for channel in 0..3 {
            assert!(
                (a[channel] - b[channel]).abs() < 0.05,
                "layer {layer} channel {channel}: {a:?} against {b:?}"
            );
        }
    }
    let json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../apps/desktop/web/models/base-slime.json"
    ))
    .unwrap();
    let near = |got: f32, want: &serde_json::Value, tolerance: f32| {
        (got - want.as_f64().unwrap() as f32).abs() <= tolerance
    };
    assert!(
        near(look.radius, &json["body"]["radius"], 0.5) || Genome::default().stage() == Stage::Baby
    );
    assert!(near(look.opacity, &json["material"]["opacity"], 0.02));
    assert!(near(
        look.edge_opacity,
        &json["material"]["edgeOpacity"],
        0.02
    ));
    assert!(near(look.glow, &json["material"]["glow"], 0.02));
    assert!(near(
        look.core_strength,
        &json["material"]["coreStrength"],
        0.02
    ));
    assert!(near(
        look.base_width,
        &json["body"]["shape"]["baseWidth"],
        0.02
    ));
    assert!(near(
        look.breathing_speed,
        &json["body"]["breathing"]["speed"],
        0.02
    ));
    assert!(near(look.eye_width, &json["face"]["eyes"]["width"], 0.002));
    assert!(near(
        look.eye_separation,
        &json["face"]["eyes"]["separation"],
        0.002
    ));
    assert!(near(look.eye_aspect, &json["face"]["eyes"]["aspect"], 0.02));
    assert_eq!(
        look.bubbles as u64,
        json["bubbles"]["count"].as_u64().unwrap()
    );
    assert_eq!(look.blush, json["face"]["blush"].as_bool().unwrap());
}

#[test]
fn every_look_stays_inside_what_the_page_model_accepts() {
    for life in 0..20u64 {
        let mut days = Days(life + 1);
        let mut genome = Genome::newborn(life);
        for night in 0..150u64 {
            genome.digest(&days.day(), night);
            let l = genome.look();
            assert!(
                (24.0..=160.0).contains(&l.radius) && l.radius <= 70.0,
                "radius {}",
                l.radius
            );
            assert!((0.2..=0.95).contains(&l.opacity), "opacity {}", l.opacity);
            assert!(
                (0.0..=0.6).contains(&l.edge_opacity)
                    && (0.0..=0.8).contains(&l.glow)
                    && (0.0..=1.0).contains(&l.core_strength)
            );
            assert!(l.bubbles <= 12);
            assert!(
                (0.8..=1.4).contains(&l.base_width) && (0.0..=6.0).contains(&l.breathing_speed)
            );
            assert!(
                (0.02..=0.12).contains(&l.eye_width)
                    && (0.08..=0.3).contains(&l.eye_separation)
                    && (0.6..=2.4).contains(&l.eye_aspect)
            );
        }
    }
}

#[test]
fn a_baby_is_smaller_than_the_same_slime_grown_up() {
    let mut genome = Genome::newborn(3);
    let baby = genome.look();
    genome.nights = 30;
    let adult = genome.look();
    assert_eq!(baby.stage, "baby");
    assert_eq!(adult.stage, "adult");
    assert!(baby.radius < adult.radius * 0.8);
}

#[test]
fn a_stored_genome_that_breaks_the_harmony_rules_is_repaired_on_load() {
    let mut value: serde_json::Value = serde_json::from_str(&Genome::newborn(1).to_json()).unwrap();
    value["genes"][Gene::CoreShift.index()] = serde_json::json!(1.0);
    value["genes"][Gene::Lightness.index()] = serde_json::json!(0.0);
    let loaded = Genome::from_json(&value.to_string()).unwrap();
    assert!(check(&loaded).is_good(), "{:?}", check(&loaded));
}

/// Regenerates the data of `apps/desktop/web/dev/genome-preview.html`:
/// `cargo test -p slimy-core --test genome write_preview_looks -- --ignored`
#[test]
#[ignore = "writes apps/desktop/web/dev/genome-looks.json for the preview page"]
fn write_preview_looks() {
    let live = |seed: u64, nights: u64, day: Signals| {
        let mut genome = Genome::newborn(seed);
        for night in 0..nights {
            genome.digest(&day, seed * 1_000 + night);
        }
        genome
    };
    let loving = Signals {
        affection: 1.0,
        curiosity: 0.4,
        ..Signals::default()
    };
    let nocturnal = Signals {
        night_use: 1.0,
        curiosity: 0.2,
        ..Signals::default()
    };
    let lively = Signals {
        energy: 1.0,
        tenacity: 0.6,
        ..Signals::default()
    };
    let cells = [
        ("newborn seed 1", live(1, 0, Signals::default())),
        ("newborn seed 2", live(2, 0, Signals::default())),
        ("newborn seed 3", live(3, 0, Signals::default())),
        ("seed 3, 12 loving nights", live(3, 12, loving)),
        ("seed 3, 40 loving nights", live(3, 40, loving)),
        ("seed 3, 40 nocturnal nights", live(3, 40, nocturnal)),
        ("seed 3, 40 lively nights", live(3, 40, lively)),
        (
            "seed 9, 120 mixed nights",
            live(
                9,
                120,
                Signals {
                    affection: 0.5,
                    energy: 0.5,
                    curiosity: 0.5,
                    night_use: 0.5,
                    tenacity: 0.5,
                },
            ),
        ),
    ];
    let json: Vec<serde_json::Value> = cells
        .iter()
        .map(|(label, genome)| serde_json::json!({ "label": label, "look": genome.look() }))
        .collect();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/web/dev/genome-looks.json"
    );
    std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
}

#[test]
fn hatchlings_from_neighbouring_seeds_do_not_look_alike() {
    let hue = |seed: u64| Genome::newborn(seed).get(Gene::SkinHue);
    let mut close = 0;
    for seed in 0..60u64 {
        let d = gene_distance(Gene::SkinHue, hue(seed), hue(seed + 1));
        if d < 0.08 {
            close += 1;
        }
    }
    assert!(
        close <= 12,
        "{close} of 60 neighbouring seed pairs gave nearly the same hue"
    );
    let mut hues: Vec<f32> = (0..200u64).map(hue).collect();
    hues.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let widest_gap = hues.windows(2).map(|w| w[1] - w[0]).fold(0.0, f32::max);
    assert!(
        widest_gap < 0.12,
        "hues leave a hole of {widest_gap} in the colour wheel"
    );
}

#[test]
fn material_genes_are_exposed_independently_in_the_look() {
    let materials = [
        Gene::Stripes,
        Gene::Honey,
        Gene::Crystal,
        Gene::Fur,
        Gene::Lava,
        Gene::Stars,
    ];
    for (selected, gene) in materials.iter().enumerate() {
        let mut genome = Genome::default();
        genome.set(*gene, 0.7);
        let layers = genome.look().layers;
        let values = [
            layers.stripes,
            layers.honey,
            layers.crystal,
            layers.fur,
            layers.lava,
            layers.stars,
        ];
        for (index, value) in values.iter().enumerate() {
            assert_eq!(*value, if index == selected { 0.7 } else { 0.0 });
        }
    }
}
