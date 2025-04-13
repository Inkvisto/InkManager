use crate::{
    chars_sets::DIGIT,
    error::ErrorKind::{
        self, InvalidCharacter, InvalidDecimalOctet, TooFewAddressParts, TooManyAddressParts,
        TruncatedHost,
    },
    uri::codec::Context,
};

struct Shared {
    num_groups: usize,
    octet_buffer: String,
}

enum State {
    NotInOctet(Shared),
    ExpectDigitOrDot(Shared),
}

impl State {
    fn finalize(self) -> Result<(), ErrorKind> {
        match self {
            Self::NotInOctet(_) => Err(TruncatedHost),
            Self::ExpectDigitOrDot(state) => Self::finalize_expect_digit_or_dot(state),
        }
    }

    fn finalize_expect_digit_or_dot(state: Shared) -> Result<(), ErrorKind> {
        let mut state = state;
        if !state.octet_buffer.is_empty() {
            state.num_groups += 1;
            if state.octet_buffer.parse::<u8>().is_err() {
                return Err(InvalidDecimalOctet);
            }
        }
        match state.num_groups {
            4 => Ok(()),
            n if n < 4 => Err(TooFewAddressParts),
            _ => Err(TooManyAddressParts),
        }
    }

    fn new() -> Self {
        Self::NotInOctet(Shared {
            num_groups: 0,
            octet_buffer: String::new(),
        })
    }

    fn next(self, c: char) -> Result<Self, ErrorKind> {
        match self {
            Self::NotInOctet(state) => Self::next_not_in_octet(state, c),
            Self::ExpectDigitOrDot(state) => Self::next_expect_digit_or_dot(state, c),
        }
    }

    fn next_not_in_octet(state: Shared, c: char) -> Result<Self, ErrorKind> {
        let mut state = state;
        if DIGIT.contains(&c) {
            state.octet_buffer.push(c);
            Ok(Self::ExpectDigitOrDot(state))
        } else {
            Err(InvalidCharacter(Context::Ipv4Address))
        }
    }

    fn next_expect_digit_or_dot(state: Shared, c: char) -> Result<Self, ErrorKind> {
        let mut state = state;
        if c == '.' {
            state.num_groups += 1;
            if state.num_groups > 4 {
                return Err(TooManyAddressParts);
            }
            if state.octet_buffer.parse::<u8>().is_err() {
                return Err(InvalidDecimalOctet);
            }
            state.octet_buffer.clear();
            Ok(Self::NotInOctet(state))
        } else if DIGIT.contains(&c) {
            state.octet_buffer.push(c);
            Ok(Self::ExpectDigitOrDot(state))
        } else {
            Err(InvalidCharacter(Context::Ipv4Address))
        }
    }
}

pub fn validate_ipv4_address<T>(address: T) -> Result<(), ErrorKind>
where
    T: AsRef<str>,
{
    address
        .as_ref()
        .chars()
        .try_fold(State::new(), State::next)?
        .finalize()
}
