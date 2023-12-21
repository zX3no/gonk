//! Ripped from <https://github.com/dguo/strsim-rs>
use std::cmp::{max, min};

pub fn jaro_winkler(a: &str, b: &str) -> f64 {
    let jaro_distance = generic_jaro(a, b);

    // Don't limit the length of the common prefix
    let prefix_length = a
        .chars()
        .zip(b.chars())
        .take_while(|(a_elem, b_elem)| a_elem == b_elem)
        .count();

    let jaro_winkler_distance =
        jaro_distance + (0.08 * prefix_length as f64 * (1.0 - jaro_distance));

    jaro_winkler_distance.clamp(0.0, 1.0)
}

pub fn generic_jaro(a: &str, b: &str) -> f64 {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    // The check for lengths of one here is to prevent integer overflow when
    // calculating the search range.
    if a_len == 0 && b_len == 0 {
        return 1.0;
    } else if a_len == 0 || b_len == 0 {
        return 0.0;
    } else if a_len == 1 && b_len == 1 {
        return if a.chars().eq(b.chars()) { 1.0 } else { 0.0 };
    }

    let search_range = (max(a_len, b_len) / 2) - 1;

    let mut b_consumed = vec![false; b_len];
    let mut matches = 0.0;

    let mut transpositions = 0.0;
    let mut b_match_index = 0;

    for (i, a_elem) in a.chars().enumerate() {
        let min_bound =
            // prevent integer wrapping
            if i > search_range {
                max(0, i - search_range)
            } else {
                0
            };

        let max_bound = min(b_len - 1, i + search_range);

        if min_bound > max_bound {
            continue;
        }

        for (j, b_elem) in b.chars().enumerate() {
            if min_bound <= j && j <= max_bound && a_elem == b_elem && !b_consumed[j] {
                b_consumed[j] = true;
                matches += 1.0;

                if j < b_match_index {
                    transpositions += 1.0;
                }
                b_match_index = j;

                break;
            }
        }
    }

    if matches == 0.0 {
        0.0
    } else {
        (1.0 / 3.0)
            * ((matches / a_len as f64)
                + (matches / b_len as f64)
                + ((matches - transpositions) / matches))
    }
}
