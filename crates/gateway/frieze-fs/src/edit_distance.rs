//! A small edit-distance helper for "did you mean ...?" suggestions.
//!
//! When a configuration table contains an unknown key, the closest
//! known key is suggested if it is plausibly a typo. The candidate set
//! is a handful of short literals, so a plain Levenshtein computation
//! is more than fast enough — no dependency is warranted for this.

/// Returns the known key closest to `unknown`, when close enough to be
/// a plausible typo.
///
/// "Close enough" means a Levenshtein distance of at most one third of
/// the unknown key's length, rounded up — long keys may drift further
/// than short ones before a suggestion stops being helpful. Ties go to
/// the earliest candidate in `known`.
pub(crate) fn suggest<'a>(unknown: &str, known: &[&'a str]) -> Option<&'a str> {
    let threshold = unknown.chars().count().div_ceil(3);
    known
        .iter()
        .map(|candidate| (levenshtein(unknown, candidate), *candidate))
        .filter(|(distance, _)| *distance <= threshold)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// The Levenshtein distance between `a` and `b`, over `char`s, with
/// the classic single-row dynamic program.
fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    // `row[j]` holds the distance between the prefix of `a` consumed
    // so far and the first `j` chars of `b`.
    let mut row: Vec<usize> = (0..=b_chars.len()).collect();
    for (i, a_char) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, b_char) in b_chars.iter().enumerate() {
            let substitution = diagonal + usize::from(a_char != *b_char);
            diagonal = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_covers_the_classic_cases() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("partial", "partial"), 0);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        // One substitution.
        assert_eq!(levenshtein("nome", "name"), 1);
        // A transposition costs two single-char edits.
        assert_eq!(levenshtein("parital", "partial"), 2);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn suggests_the_closest_known_key_for_a_plausible_typo() {
        let known = ["name", "partial", "output"];
        assert_eq!(suggest("parital", &known), Some("partial"));
        assert_eq!(suggest("ouput", &known), Some("output"));
        assert_eq!(suggest("nme", &known), Some("name"));
    }

    #[test]
    fn stays_silent_when_nothing_is_close() {
        let known = ["name", "partial", "output"];
        assert_eq!(suggest("description", &known), None);
        assert_eq!(suggest("x", &known), None);
    }
}
