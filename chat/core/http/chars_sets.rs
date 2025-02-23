use std::{collections::HashSet, sync::LazyLock};

pub const ALPHA: LazyLock<HashSet<char>> = LazyLock::new(|| ('a'..='z').chain('A'..='Z').collect());
pub const DIGIT: LazyLock<HashSet<char>> = LazyLock::new(|| ('0'..='9').collect());
pub const SCHEME: LazyLock<HashSet<char>> = LazyLock::new(|| {
    ALPHA
        .iter()
        .chain(DIGIT.iter())
        .chain(['+', '-', '.'].iter())
        .copied()
        .collect()
});
