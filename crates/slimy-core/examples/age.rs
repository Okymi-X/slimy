//! Dev tool: plays N nights of the real digestion on a copy of a soul file, one busy day after another.
//! `cargo run -p slimy-core --example age -- <in.json> <out.json> <nights>`

use slimy_core::{DayLog, Soul};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (input, output, nights) = (&args[1], &args[2], args[3].parse::<i64>().expect("nights"));
    let mut soul =
        Soul::from_json(&std::fs::read_to_string(input).expect("read")).expect("valid soul");
    let start = soul.day.day.max(1);
    for n in 0..nights {
        let busy = DayLog {
            day: start + n,
            caress_seconds: 420.0,
            throws: 26,
            answered: 3,
            messages: 8,
            active_seconds: 5400.0,
            night_seconds: 1800.0,
            ..DayLog::default()
        };
        soul.day = busy;
        soul.settle_day(start + n + 1);
    }
    std::fs::write(output, soul.to_json()).expect("write");
    println!("nights played: {}", nights);
}
