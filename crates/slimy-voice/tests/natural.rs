use slimy_voice::*;

fn base() -> Voice {
    voice_for(
        "fr",
        Traits {
            affection: 0.5,
            curiosity: 0.5,
            energy: 0.5,
            size: 0.5,
        },
        Style::Auto,
    )
}

#[test]
fn feelings_colour_the_voice_within_safe_bounds() {
    let calm = base();
    let joy = base().with_mood(Some("joy"), 0.8, 0.9);
    let sleepy = base().with_mood(Some("sleepy"), -0.2, 0.1);
    let sulk = base().with_mood(Some("sulk"), -0.8, 0.2);
    assert!(joy.speed > calm.speed && joy.pitch > calm.pitch);
    assert!(sleepy.speed < calm.speed && sleepy.pitch < calm.pitch);
    assert!(sulk.pitch < calm.pitch);
    assert_eq!(
        base().with_mood(None, 0.0, 0.3).speaker,
        calm.speaker,
        "the voice itself never changes"
    );
    for v in [
        joy,
        sleepy,
        sulk,
        base().with_mood(Some("surprised"), f32::NAN, f32::INFINITY),
        base().with_mood(Some("???"), 9.0, -9.0),
    ] {
        assert!(
            (0.8..=1.3).contains(&v.speed) && (0.9..=1.25).contains(&v.pitch),
            "{v:?}"
        );
    }
}

#[test]
fn symbols_are_read_as_words_in_each_language() {
    assert_eq!(
        speakable_in("Il fait 24°C et 50 % d'humidité", "fr"),
        "Il fait 24 degrés et 50 pour cent d'humidité."
    );
    assert_eq!(
        speakable_in("It is 24°C & 50% humid", "en"),
        "It is 24 degrees and 50 percent humid."
    );
    assert_eq!(
        speakable_in("Vitesse 30 km/h", "fr"),
        "Vitesse 30 kilomètres heure."
    );
    assert_eq!(speakable_in("A/B", "en"), "A slash B.");
}

#[test]
fn text_is_cut_into_sentences_without_choppy_scraps() {
    // short scraps join their neighbour, so there is no clip of two words
    assert_eq!(
        sentences(
            "Salut ! Je suis content de te voir aujourd'hui. Et toi ?",
            20
        ),
        vec!["Salut ! Je suis content de te voir aujourd'hui. Et toi ?"]
    );
    // long sentences stay apart so the first can start playing early
    assert_eq!(
        sentences(
            "Voici une première phrase assez longue. Voici une seconde phrase tout aussi longue.",
            20
        ),
        vec![
            "Voici une première phrase assez longue.",
            "Voici une seconde phrase tout aussi longue."
        ]
    );
    assert_eq!(
        sentences("Pas de ponctuation du tout", 20),
        vec!["Pas de ponctuation du tout"]
    );
    assert!(sentences("   ", 20).is_empty());
    assert!(sentences("Une.", 20).concat().ends_with('.'));
}

fn tone(seconds: f32, rate: u32, amplitude: f32) -> Vec<f32> {
    (0..(seconds * rate as f32) as usize)
        .map(|i| amplitude * (i as f32 * 0.2).sin())
        .collect()
}

#[test]
fn a_clip_is_trimmed_faded_and_set_to_a_steady_loudness() {
    let rate = 24_000;
    let mut clip = silence(rate, 0.4);
    clip.extend(tone(1.0, rate, 0.02));
    clip.extend(silence(rate, 0.5));
    let done = finish(&clip, rate);
    let seconds = done.len() as f32 / rate as f32;
    assert!(
        (1.05..1.25).contains(&seconds),
        "trimmed to the sound plus a breath, got {seconds:.2} s"
    );
    assert!(
        done[0].abs() < 0.01 && done[done.len() - 1].abs() < 0.01,
        "faded at both ends"
    );
    let rms = (done.iter().map(|s| s * s).sum::<f32>() / done.len() as f32).sqrt();
    assert!((0.07..0.13).contains(&rms), "steady loudness, rms {rms:.3}");
    let loud = finish(&tone(1.0, rate, 0.95), rate);
    assert!(loud.iter().all(|s| s.abs() <= 0.96), "never clips");
    assert!(finish(&[], rate).is_empty());
    assert_eq!(
        finish(&vec![0.0; 100], rate),
        vec![0.0; 100],
        "silence stays silence"
    );
}

#[test]
fn a_pause_has_the_asked_length() {
    assert_eq!(silence(24_000, 0.25).len(), 6_000);
    assert!(silence(24_000, 0.0).is_empty());
}
