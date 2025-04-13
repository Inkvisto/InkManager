use std::{collections::HashSet, sync::LazyLock};

pub const ALPHA: LazyLock<HashSet<char>> = LazyLock::new(|| ('a'..='z').chain('A'..='Z').collect());
pub const DIGIT: LazyLock<HashSet<char>> = LazyLock::new(|| ('0'..='9').collect());
pub const HEXDIG: LazyLock<HashSet<char>> = LazyLock::new(|| {
    DIGIT
        .iter()
        .copied()
        .chain('A'..='F')
        .chain('a'..='f')
        .collect()
});

pub const SCHEME: LazyLock<HashSet<char>> = LazyLock::new(|| {
    ALPHA
        .iter()
        .chain(DIGIT.iter())
        .chain(['+', '-', '.'].iter())
        .copied()
        .collect()
});
pub const UNRESERVED: LazyLock<HashSet<char>> = LazyLock::new(|| {
    ALPHA
        .iter()
        .chain(DIGIT.iter())
        .chain(['-', '.', '_', '~'].iter())
        .copied()
        .collect()
});
pub const SUB_DELIMS: LazyLock<HashSet<char>> = LazyLock::new(|| {
    ['!', '$', '&', '\'', '(', ')', '*', '+', ',', ';', '=']
        .iter()
        .copied()
        .collect()
});
pub const USER_INFO_NOT_PCT_ENCODED: LazyLock<HashSet<char>> = LazyLock::new(|| {
    UNRESERVED
        .iter()
        .chain(SUB_DELIMS.iter())
        .chain([':'].iter())
        .copied()
        .collect()
});

pub const REG_NAME_NOT_PCT_ENCODED: LazyLock<HashSet<char>> = LazyLock::new(|| {
    UNRESERVED
        .iter()
        .chain(SUB_DELIMS.iter())
        .copied()
        .collect()
});

pub const IPV_FUTURE_LAST_PART: LazyLock<HashSet<char>> = LazyLock::new(|| {
    UNRESERVED
        .iter()
        .chain(SUB_DELIMS.iter())
        .chain([':'].iter())
        .copied()
        .collect()
});
