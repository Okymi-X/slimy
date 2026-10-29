/// Longest text spoken in one go, in characters: the slime says a sentence or two, never a lecture.
pub const MAX_SPOKEN: usize = 260;

fn is_pictograph(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0xFE00..=0xFE0F | 0x200D | 0x20E3)
}

/// The text as it should be read aloud: no emoji, markdown marks or addresses, tidy spaces, cut at a sentence end.
pub fn speakable(text: &str) -> String {
    let mut words = Vec::new();
    for word in text.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") || word.starts_with("www.") {
            continue;
        }
        let clean: String = word
            .chars()
            .filter(|c| {
                !is_pictograph(*c) && !matches!(c, '*' | '_' | '`' | '#' | '~' | '|' | '<' | '>')
            })
            .collect();
        if !clean.is_empty() {
            words.push(clean);
        }
    }
    let mut out = words.join(" ").replace("...", ", ").replace('…', ", ");
    out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.chars().count() > MAX_SPOKEN {
        let cut: String = out.chars().take(MAX_SPOKEN).collect();
        out = match cut.rfind(['.', '!', '?']) {
            Some(i) if i > MAX_SPOKEN / 3 => cut[..=i].to_string(),
            _ => cut.trim_end_matches([',', ' ']).to_string(),
        };
    }
    let out = out.trim().trim_end_matches(',').trim().to_string();
    if !out.is_empty() && !out.ends_with(['.', '!', '?']) {
        return format!("{out}.");
    }
    out
}

/// Words said instead of symbols, per language, so "24 %" and "A & B" are read as a person would.
fn spoken_symbols(text: &str, french: bool) -> String {
    let (percent, and, plus, equals, degrees, per_hour) = if french {
        (
            " pour cent",
            " et ",
            " plus ",
            " égale ",
            " degrés",
            " kilomètres heure",
        )
    } else {
        (
            " percent",
            " and ",
            " plus ",
            " equals ",
            " degrees",
            " kilometers per hour",
        )
    };
    text.replace("°C", degrees)
        .replace("°F", degrees)
        .replace('°', degrees)
        .replace("km/h", per_hour)
        .replace('%', percent)
        .replace('&', and)
        .replace(" + ", plus)
        .replace(" = ", equals)
        .replace("etc.", "et cetera.")
        .replace('/', if french { " sur " } else { " slash " })
}

/// Like `speakable`, also reading symbols aloud in the given language ("fr" or anything else for English).
pub fn speakable_in(text: &str, language: &str) -> String {
    speakable(&spoken_symbols(text, language.starts_with("fr")))
}

/// Cuts a spoken text into sentences, merging tiny ones into their neighbour so no clip is shorter than `min_chars`
/// (very short clips sound choppy). Every sentence keeps its end mark.
pub fn sentences(text: &str, min_chars: usize) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        current.push(c);
        if matches!(c, '.' | '!' | '?') {
            let sentence = std::mem::take(&mut current).trim().to_string();
            if !sentence.is_empty() {
                parts.push(sentence);
            }
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    let mut out: Vec<String> = Vec::new();
    for part in parts {
        match out.last_mut() {
            Some(last) if last.chars().count() < min_chars => {
                last.push(' ');
                last.push_str(&part);
            }
            _ => out.push(part),
        }
    }
    if out.len() > 1 && out.last().is_some_and(|l| l.chars().count() < min_chars) {
        let tail = out.pop().expect("checked");
        let last = out.last_mut().expect("another sentence remains");
        last.push(' ');
        last.push_str(&tail);
    }
    out
}
