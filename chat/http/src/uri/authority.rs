use crate::{
    chars_sets::{
        HEXDIG, IPV_FUTURE_LAST_PART, REG_NAME_NOT_PCT_ENCODED, USER_INFO_NOT_PCT_ENCODED,
    },
    error::ErrorKind::{
        self, InvalidAuthority, InvalidCharacter, InvalidPortNumber, TruncatedHost,
    },
    uri::{
        codec::{decode_element, Context, PercentEncodedCharacterDecoder},
        ipv6::validate_ipv6_address,
    },
};

#[derive(Debug)]
pub struct Authority {
    userinfo: Option<Vec<u8>>,
    host: Vec<u8>,
    port: Option<u16>,
}

struct Shared {
    host: Vec<u8>,
    host_is_reg_name: bool,
    ipv6_address: String,
    pec_decoder: PercentEncodedCharacterDecoder,
    port_string: String,
}

enum State {
    NotIpLiteral(Shared),
    PercentEncodedCharacter(Shared),
    Ipv6Address(Shared),
    IpvFutureNumber(Shared),
    IpvFutureBody(Shared),
    GarbageCheck(Shared),
    Port(Shared),
}

impl Authority {
    pub fn parse<T>(authority_string: T) -> Result<Self, ErrorKind>
    where
        T: AsRef<str>,
    {
        let (userinfo, host_port_string) = Self::parse_userinfo(authority_string.as_ref())?;
        let (host, port) = parse_host_port(host_port_string)?;
        Ok(Self {
            userinfo,
            host,
            port,
        })
    }

    fn parse_userinfo(authority: &str) -> Result<(Option<Vec<u8>>, &str), ErrorKind> {
        Ok(match authority.find('@') {
            Some(delimiter) => (
                Some(decode_element(
                    &authority[0..delimiter],
                    &USER_INFO_NOT_PCT_ENCODED,
                    Context::Userinfo,
                )?),
                &authority[delimiter + 1..],
            ),
            None => (None, authority),
        })
    }
}

impl State {
    fn finalize(self) -> Result<(Vec<u8>, Option<u16>), ErrorKind> {
        match self {
            Self::PercentEncodedCharacter(_)
            | Self::Ipv6Address(_)
            | Self::IpvFutureNumber(_)
            | Self::IpvFutureBody(_) => {
                // truncated or ended early
                Err(TruncatedHost)
            }
            Self::NotIpLiteral(state) | Self::GarbageCheck(state) | Self::Port(state) => {
                let mut state = state;
                if state.host_is_reg_name {
                    state.host.make_ascii_lowercase();
                }
                let port = if state.port_string.is_empty() {
                    None
                } else {
                    match state.port_string.parse::<u16>() {
                        Ok(port) => Some(port),
                        Err(error) => {
                            return Err(InvalidPortNumber(error));
                        }
                    }
                };
                Ok((state.host, port))
            }
        }
    }

    fn new(host_port_string: &str) -> (Self, &str) {
        let mut shared = Shared {
            host: Vec::<u8>::new(),
            host_is_reg_name: false,
            ipv6_address: String::new(),
            pec_decoder: PercentEncodedCharacterDecoder::new(),
            port_string: String::new(),
        };
        let mut host_port_string = host_port_string;
        if host_port_string.starts_with("[v") {
            host_port_string = &host_port_string[2..];
            shared.host.push(b'v');
            (Self::IpvFutureNumber(shared), host_port_string)
        } else if host_port_string.starts_with('[') {
            host_port_string = &host_port_string[1..];
            (Self::Ipv6Address(shared), host_port_string)
        } else {
            shared.host_is_reg_name = true;
            (Self::NotIpLiteral(shared), host_port_string)
        }
    }

    fn next(self, c: char) -> Result<Self, ErrorKind> {
        match self {
            Self::NotIpLiteral(state) => Self::next_not_ip_literal(state, c),
            Self::PercentEncodedCharacter(state) => Self::next_percent_encoded_character(state, c),
            Self::Ipv6Address(state) => Self::next_ipv6_address(state, c),
            Self::IpvFutureNumber(state) => Self::next_ipv_future_number(state, c),
            Self::IpvFutureBody(state) => Self::next_ipv_future_body(state, c),
            Self::GarbageCheck(state) => Self::next_garbage_check(state, c),
            Self::Port(state) => Ok(Self::next_port(state, c)),
        }
    }

    fn next_not_ip_literal(state: Shared, c: char) -> Result<Self, ErrorKind> {
        let mut state = state;
        if c == '%' {
            Ok(Self::PercentEncodedCharacter(state))
        } else if c == ':' {
            Ok(Self::Port(state))
        } else if REG_NAME_NOT_PCT_ENCODED.contains(&c) {
            state.host.push(u8::try_from(c as u32).unwrap());
            Ok(Self::NotIpLiteral(state))
        } else {
            Err(InvalidCharacter(Context::Host))
        }
    }

    fn next_percent_encoded_character(state: Shared, c: char) -> Result<Self, ErrorKind> {
        let mut state = state;
        // We can't use `Option::map_or` (or `Option::map_or_else`, for similar
        // reasons) in this case because the closure would take ownership of
        // `state`, preventing it from being used to construct the default
        // value.
        #[allow(clippy::option_if_let_else)]
        if let Some(ci) = state.pec_decoder.next(c)? {
            state.host.push(ci);
            Ok(Self::NotIpLiteral(state))
        } else {
            Ok(Self::PercentEncodedCharacter(state))
        }
    }

    fn next_ipv6_address(state: Shared, c: char) -> Result<Self, ErrorKind> {
        let mut state = state;
        if c == ']' {
            validate_ipv6_address(&state.ipv6_address)?;
            state.host = state
                .ipv6_address
                .chars()
                .map(|c| u8::try_from(c as u32).unwrap())
                .collect();
            Ok(Self::GarbageCheck(state))
        } else {
            state.ipv6_address.push(c);
            Ok(Self::Ipv6Address(state))
        }
    }

    fn next_ipv_future_number(state: Shared, c: char) -> Result<Self, ErrorKind> {
        let mut state = state;
        if c == '.' {
            state.host.push(b'.');
            Ok(Self::IpvFutureBody(state))
        } else if c == ']' {
            Err(TruncatedHost)
        } else if HEXDIG.contains(&c) {
            state.host.push(u8::try_from(c as u32).unwrap());
            Ok(Self::IpvFutureNumber(state))
        } else {
            Err(InvalidCharacter(Context::IpvFuture))
        }
    }

    fn next_ipv_future_body(state: Shared, c: char) -> Result<Self, ErrorKind> {
        let mut state = state;
        if c == ']' {
            Ok(Self::GarbageCheck(state))
        } else if IPV_FUTURE_LAST_PART.contains(&c) {
            state.host.push(u8::try_from(c as u32).unwrap());
            Ok(Self::IpvFutureBody(state))
        } else {
            Err(InvalidCharacter(Context::IpvFuture))
        }
    }

    fn next_garbage_check(state: Shared, c: char) -> Result<Self, ErrorKind> {
        // illegal to have anything else, unless it's a colon,
        // in which case it's a port delimiter
        if c == ':' {
            Ok(Self::Port(state))
        } else {
            Err(InvalidCharacter(Context::Host))
        }
    }

    fn next_port(state: Shared, c: char) -> Self {
        let mut state = state;
        state.port_string.push(c);
        Self::Port(state)
    }
}

pub fn parse_host_port<T>(host_port_string: T) -> Result<(Vec<u8>, Option<u16>), ErrorKind>
where
    T: AsRef<str>,
{
    let (machine, host_port_string) = State::new(host_port_string.as_ref());
    host_port_string
        .chars()
        .try_fold(machine, State::next)?
        .finalize()
}
