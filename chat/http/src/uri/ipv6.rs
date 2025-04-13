use crate::{
    chars_sets::{DIGIT, HEXDIG},
    error::ErrorKind::{
        self, InvalidCharacter, TooFewAddressParts, TooManyAddressParts, TooManyDigits,
        TooManyDoubleColons, TruncatedHost,
    },
    uri::{codec::Context, ipv4::validate_ipv4_address},
};

enum MachineExitStatus {
    Error(ErrorKind),
    Ipv4Trailer(Shared),
}

impl From<ErrorKind> for MachineExitStatus {
    fn from(error: ErrorKind) -> Self {
        MachineExitStatus::Error(error)
    }
}

struct Shared {
    address: String,
    num_groups: usize,
    num_digits: usize,
    double_colon_encountered: bool,
    potential_ipv4_address_start: usize,
}

enum State {
    NoGroupsYet(Shared),
    ColonButNoGroupsYet(Shared),
    AfterDoubleColon(Shared),
    InGroupNotIpv4(Shared),
    InGroupCouldBeIpv4(Shared),
    InGroupIpv4(Shared),
    ColonAfterGroup(Shared),
}

impl State {
    fn finalize(mut self) -> Result<(), ErrorKind> {
        match &mut self {
            Self::InGroupNotIpv4(state) | Self::InGroupCouldBeIpv4(state) => {
                // count trailing group
                state.num_groups += 1;
            }
            Self::InGroupIpv4(state) => {
                validate_ipv4_address(&state.address[state.potential_ipv4_address_start..])?;
                state.num_groups += 2;
            }
            _ => {}
        };
        match self {
            Self::ColonButNoGroupsYet(_) | Self::ColonAfterGroup(_) => Err(TruncatedHost),

            Self::AfterDoubleColon(state)
            | Self::InGroupNotIpv4(state)
            | Self::InGroupCouldBeIpv4(state)
            | Self::InGroupIpv4(state)
            | Self::NoGroupsYet(state) => {
                match (state.double_colon_encountered, state.num_groups) {
                    (true, n) if n <= 7 => Ok(()),
                    (false, 8) => Ok(()),
                    (false, n) if n < 8 => Err(TooFewAddressParts),
                    (_, _) => Err(TooManyAddressParts),
                }
            }
        }
    }

    fn new(address: &str) -> Self {
        Self::NoGroupsYet(Shared {
            address: address.to_string(),
            num_groups: 0,
            num_digits: 0,
            double_colon_encountered: false,
            potential_ipv4_address_start: 0,
        })
    }

    fn next(self, i: usize, c: char) -> Result<Self, MachineExitStatus> {
        match self {
            Self::NoGroupsYet(state) => Self::next_no_groups_yet(state, i, c),
            Self::ColonButNoGroupsYet(state) => Self::next_colon_but_no_groups_yet(state, c),
            Self::AfterDoubleColon(state) => Self::next_after_double_colon(state, i, c),
            Self::InGroupNotIpv4(state) => Self::next_in_group_not_ipv4(state, c),
            Self::InGroupCouldBeIpv4(state) => Self::next_in_group_could_be_ipv4(state, c),
            Self::InGroupIpv4(state) => Ok(Self::InGroupIpv4(state)),
            Self::ColonAfterGroup(state) => Self::next_colon_after_group(state, i, c),
        }
    }

    fn next_no_groups_yet(state: Shared, i: usize, c: char) -> Result<Self, MachineExitStatus> {
        let mut state = state;
        if c == ':' {
            Ok(Self::ColonButNoGroupsYet(state))
        } else if DIGIT.contains(&c) {
            state.potential_ipv4_address_start = i;
            state.num_digits = 1;
            Ok(Self::InGroupCouldBeIpv4(state))
        } else if HEXDIG.contains(&c) {
            state.num_digits = 1;
            Ok(Self::InGroupNotIpv4(state))
        } else {
            Err(InvalidCharacter(Context::Ipv6Address).into())
        }
    }

    fn next_colon_but_no_groups_yet(state: Shared, c: char) -> Result<Self, MachineExitStatus> {
        let mut state = state;
        if c == ':' {
            state.double_colon_encountered = true;
            Ok(Self::AfterDoubleColon(state))
        } else {
            Err(InvalidCharacter(Context::Ipv6Address).into())
        }
    }

    fn next_after_double_colon(
        state: Shared,
        i: usize,
        c: char,
    ) -> Result<Self, MachineExitStatus> {
        let mut state = state;
        state.num_digits += 1;
        if state.num_digits > 4 {
            Err(TooManyDigits.into())
        } else if DIGIT.contains(&c) {
            state.potential_ipv4_address_start = i;
            Ok(Self::InGroupCouldBeIpv4(state))
        } else if HEXDIG.contains(&c) {
            Ok(Self::InGroupNotIpv4(state))
        } else {
            Err(InvalidCharacter(Context::Ipv6Address).into())
        }
    }

    fn next_in_group_not_ipv4(state: Shared, c: char) -> Result<Self, MachineExitStatus> {
        let mut state = state;
        if c == ':' {
            state.num_digits = 0;
            state.num_groups += 1;
            Ok(Self::ColonAfterGroup(state))
        } else if HEXDIG.contains(&c) {
            state.num_digits += 1;
            if state.num_digits > 4 {
                Err(TooManyDigits.into())
            } else {
                Ok(Self::InGroupNotIpv4(state))
            }
        } else {
            Err(InvalidCharacter(Context::Ipv6Address).into())
        }
    }

    fn next_in_group_could_be_ipv4(state: Shared, c: char) -> Result<Self, MachineExitStatus> {
        let mut state = state;
        if c == ':' {
            state.num_digits = 0;
            state.num_groups += 1;
            Ok(Self::ColonAfterGroup(state))
        } else if c == '.' {
            Err(MachineExitStatus::Ipv4Trailer(state))
        } else {
            state.num_digits += 1;
            if state.num_digits > 4 {
                Err(TooManyDigits.into())
            } else if DIGIT.contains(&c) {
                Ok(Self::InGroupCouldBeIpv4(state))
            } else if HEXDIG.contains(&c) {
                Ok(Self::InGroupNotIpv4(state))
            } else {
                Err(InvalidCharacter(Context::Ipv6Address).into())
            }
        }
    }

    fn next_colon_after_group(state: Shared, i: usize, c: char) -> Result<Self, MachineExitStatus> {
        let mut state = state;
        if c == ':' {
            if state.double_colon_encountered {
                Err(TooManyDoubleColons.into())
            } else {
                state.double_colon_encountered = true;
                Ok(Self::AfterDoubleColon(state))
            }
        } else if DIGIT.contains(&c) {
            state.potential_ipv4_address_start = i;
            state.num_digits += 1;
            Ok(Self::InGroupCouldBeIpv4(state))
        } else if HEXDIG.contains(&c) {
            state.num_digits += 1;
            Ok(Self::InGroupNotIpv4(state))
        } else {
            Err(InvalidCharacter(Context::Ipv6Address).into())
        }
    }
}

pub fn validate_ipv6_address<T>(address: T) -> Result<(), ErrorKind>
where
    T: AsRef<str>,
{
    let address = address.as_ref();
    address
        .char_indices()
        .try_fold(State::new(address), |machine, (i, c)| machine.next(i, c))
        .or_else(|machine_exit_status| match machine_exit_status {
            MachineExitStatus::Ipv4Trailer(state) => Ok(State::InGroupIpv4(state)),
            MachineExitStatus::Error(error) => Err(error),
        })?
        .finalize()
}
