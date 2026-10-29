use slimy_voice::*;

const RATE: u32 = 16_000;

fn tone(seconds: f32, amplitude: f32) -> Vec<f32> {
    (0..(seconds * RATE as f32) as usize)
        .map(|i| amplitude * (i as f32 * 0.3).sin())
        .collect()
}

fn run(parts: &[(f32, f32)]) -> (Heard, f32) {
    let mut gate = Endpointer::new(RATE);
    let mut at = 0.0;
    for (seconds, amplitude) in parts {
        for chunk in tone(*seconds, *amplitude).chunks(1600) {
            at += chunk.len() as f32 / RATE as f32;
            if gate.feed(chunk) != Heard::Waiting && gate.state() != Heard::Speaking {
                return (gate.state(), at);
            }
        }
    }
    (gate.state(), at)
}

#[test]
fn speech_followed_by_a_pause_ends_the_recording() {
    // 0.7 s of room noise, 1.5 s of speech, then silence
    let (state, at) = run(&[(0.7, 0.004), (1.5, 0.2), (3.0, 0.004)]);
    assert_eq!(state, Heard::Finished);
    assert!((3.1..3.5).contains(&at), "stopped at {at:.2} s");
}

#[test]
fn a_noisy_room_is_not_speech_and_nobody_talking_gives_up() {
    let (state, _) = run(&[(12.0, 0.03)]);
    assert_eq!(state, Heard::GaveUp);
    let (state, at) = run(&[(12.0, 0.0)]);
    assert_eq!((state, at > 9.9), (Heard::GaveUp, true));
}

#[test]
fn speech_louder_than_the_room_is_heard_and_a_click_is_not() {
    let mut gate = Endpointer::new(RATE);
    for chunk in tone(0.7, 0.02).chunks(1600) {
        gate.feed(chunk);
    }
    assert_eq!(
        gate.feed(&tone(0.05, 0.5)),
        Heard::Waiting,
        "a click is too short"
    );
    for chunk in tone(0.5, 0.3).chunks(1600) {
        gate.feed(chunk);
    }
    assert_eq!(gate.state(), Heard::Speaking);
}

#[test]
fn endless_talk_stops_and_the_decision_stays() {
    let (state, at) = run(&[(0.7, 0.004), (40.0, 0.2)]);
    assert_eq!(state, Heard::TooLong);
    assert!(at < 26.5);
    let mut gate = Endpointer::new(RATE);
    for chunk in tone(0.7, 0.004).chunks(1600) {
        gate.feed(chunk);
    }
    for chunk in tone(1.0, 0.3).chunks(1600) {
        gate.feed(chunk);
    }
    for chunk in tone(1.2, 0.0).chunks(1600) {
        gate.feed(chunk);
    }
    assert_eq!(gate.state(), Heard::Finished);
    assert_eq!(
        gate.feed(&tone(0.5, 0.5)),
        Heard::Finished,
        "stays finished"
    );
}

#[test]
fn loudness_is_a_root_mean_square() {
    assert_eq!(level(&[]), 0.0);
    assert!((level(&[0.5, -0.5, 0.5, -0.5]) - 0.5).abs() < 1e-6);
}

#[test]
fn the_text_is_fit_to_be_spoken() {
    assert_eq!(
        speakable("Coucou ! 😊 **Tu** vas bien ? https://x.org"),
        "Coucou ! Tu vas bien ?"
    );
    assert_eq!(speakable("Hmm... d'accord"), "Hmm, d'accord.");
    assert_eq!(speakable("   "), "");
    assert_eq!(speakable("`code` et _italique_"), "code et italique.");
    let long = "Une phrase. ".repeat(60);
    let out = speakable(&long);
    assert!(out.chars().count() <= MAX_SPOKEN && out.ends_with('.'));
}

fn traits(affection: f32, curiosity: f32, energy: f32, size: f32) -> Traits {
    Traits {
        affection,
        curiosity,
        energy,
        size,
    }
}

#[test]
fn the_voice_follows_the_character_and_the_language() {
    let french = voice_for("fr", traits(0.5, 0.5, 0.5, 0.5), Style::Auto);
    assert_eq!((french.speaker, french.name), (30, "ff_siwis"));
    let lively = voice_for("en", traits(0.5, 0.8, 0.8, 0.5), Style::Auto);
    let tender = voice_for("en", traits(0.9, 0.3, 0.3, 0.5), Style::Auto);
    let dreamy = voice_for("en", traits(0.3, 0.3, 0.2, 0.5), Style::Auto);
    assert_eq!(lively.name, "af_bella");
    assert_eq!(tender.name, "af_heart");
    assert_eq!(dreamy.name, "af_nicole");
    assert_eq!(
        voice_for("en", traits(0.5, 0.5, 0.5, 0.5), Style::Auto),
        voice_for("en", traits(0.5, 0.5, 0.5, 0.5), Style::Auto)
    );
}

#[test]
fn a_small_slime_has_a_higher_voice_and_the_style_moves_it() {
    let small = voice_for("fr", traits(0.5, 0.5, 0.5, 0.0), Style::Auto);
    let big = voice_for("fr", traits(0.5, 0.5, 0.5, 1.0), Style::Auto);
    assert!(small.pitch > big.pitch);
    let kawaii = voice_for("fr", traits(0.5, 0.5, 0.5, 0.5), Style::Kawaii);
    let calm = voice_for("fr", traits(0.5, 0.5, 0.5, 0.5), Style::Calm);
    let auto = voice_for("fr", traits(0.5, 0.5, 0.5, 0.5), Style::Auto);
    assert!(kawaii.pitch > auto.pitch && auto.pitch > calm.pitch);
    assert!(kawaii.speed > calm.speed);
    for v in [kawaii, calm, small, big] {
        assert!((0.85..=1.25).contains(&v.speed) && (0.92..=1.22).contains(&v.pitch));
    }
    let broken = voice_for(
        "en",
        traits(f32::NAN, f32::INFINITY, -4.0, 9.0),
        Style::Auto,
    );
    assert!(broken.speed.is_finite() && broken.pitch.is_finite());
}

#[test]
fn a_decaying_offset_is_removed_but_speech_stays() {
    let mut blocker = DcBlocker::new();
    // an offset that starts at 0.9 and decays over a second, with a tone riding on it
    let mut signal: Vec<f32> = (0..RATE as usize * 2)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            0.9 * (-t * 3.0).exp()
                + if t > 1.2 {
                    0.2 * (i as f32 * 0.3).sin()
                } else {
                    0.0
                }
        })
        .collect();
    blocker.process(&mut signal);
    let quiet = level(&signal[RATE as usize / 2..RATE as usize]);
    let speech = level(&signal[RATE as usize * 3 / 2..]);
    assert!(quiet < 0.06, "the offset is gone, {quiet:.3} left");
    assert!(speech > 0.1, "the tone is kept, {speech:.3}");
}

#[test]
fn a_microphone_that_starts_with_an_offset_is_still_heard() {
    let mut blocker = DcBlocker::new();
    let mut stream: Vec<f32> = (0..RATE as usize * 6)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            0.9 * (-t * 3.0).exp()
                + if (1.5..3.0).contains(&t) {
                    0.25 * (i as f32 * 0.3).sin()
                } else {
                    0.003 * (i as f32 * 0.7).sin()
                }
        })
        .collect();
    blocker.process(&mut stream);
    let mut gate = Endpointer::new(RATE);
    let mut finished = false;
    for chunk in stream.chunks(640) {
        if gate.feed(chunk) == Heard::Finished {
            finished = true;
            break;
        }
    }
    assert!(
        finished,
        "speech between 1.5 s and 3 s is found despite the start-up offset, state {:?}",
        gate.state()
    );
}

#[test]
fn a_hissing_microphone_with_spikes_does_not_hide_the_end_of_speech() {
    // ambient hiss around 0.02 with spikes to 0.045, as measured on a real laptop microphone, then speech, then hiss again
    let mut seed = 12345u32;
    let mut noise = move |scale: f32| {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        ((seed >> 16) as f32 / 65535.0 - 0.5) * 2.0 * scale
    };
    let hiss = |n: &mut dyn FnMut(f32) -> f32, seconds: f32| -> Vec<f32> {
        (0..(seconds * RATE as f32) as usize)
            .map(|_| n(0.028))
            .collect()
    };
    let mut stream = hiss(&mut noise, 1.0);
    stream.extend((0..RATE as usize * 2).map(|i| 0.22 * (i as f32 * 0.25).sin() + noise(0.028)));
    stream.extend(hiss(&mut noise, 3.0));
    let mut gate = Endpointer::new(RATE);
    let mut finished_at = None;
    let mut at = 0.0;
    for chunk in stream.chunks(640) {
        at += 640.0 / RATE as f32;
        if gate.feed(chunk) == Heard::Finished {
            finished_at = Some(at);
            break;
        }
    }
    let at = finished_at.expect("the recording ends");
    assert!(
        (3.8..4.4).contains(&at),
        "ended at {at:.2} s, speech stops at 3.0 s"
    );
}

#[test]
fn a_short_noise_is_not_taken_for_speech_and_real_speech_after_it_still_is() {
    // 0.7 s of room, a 0.2 s cough, 2 s of quiet, then 1 s of speech and silence
    let (state, _) = run(&[
        (0.7, 0.004),
        (0.2, 0.3),
        (2.0, 0.004),
        (1.0, 0.2),
        (2.5, 0.004),
    ]);
    assert_eq!(state, Heard::Finished);
    let (state, at) = run(&[(0.7, 0.004), (0.2, 0.3), (10.0, 0.004)]);
    assert_eq!(
        state,
        Heard::GaveUp,
        "a cough alone is nobody speaking (gave up at {at:.1} s)"
    );
}

#[test]
fn a_quiet_recording_is_raised_and_the_rest_is_left_alone() {
    let mut quiet: Vec<f32> = (0..1000).map(|i| 0.06 * (i as f32 * 0.2).sin()).collect();
    normalize(&mut quiet);
    let peak = quiet.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!((0.85..=0.95).contains(&peak), "peak {peak}");
    let mut hiss: Vec<f32> = vec![0.005; 100];
    normalize(&mut hiss);
    assert_eq!(hiss, vec![0.005; 100], "silence is not amplified");
    let mut loud = vec![0.95f32, -0.95, 0.5];
    normalize(&mut loud);
    assert_eq!(loud, vec![0.95, -0.95, 0.5]);
    let mut weak: Vec<f32> = vec![0.021, -0.021];
    normalize(&mut weak);
    assert!(weak[0] <= 0.021 * 15.0 + 1e-6, "gain is capped");
}

#[test]
fn the_start_of_speech_is_reported_so_the_silence_before_it_can_be_cut() {
    let mut gate = Endpointer::new(RATE);
    assert_eq!(gate.speech_began(), None);
    for chunk in tone(0.7, 0.004).chunks(640) {
        gate.feed(chunk);
    }
    for chunk in tone(1.0, 0.25).chunks(640) {
        gate.feed(chunk);
    }
    let began = gate.speech_began().expect("speech was heard");
    assert!(
        (0.65..0.85).contains(&began),
        "began at {began:.2} s, speech starts at 0.7 s"
    );
}
