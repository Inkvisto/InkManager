use crate::error::ErrorKind::{self, InvalidCharacter, InvalidPercentEncoding};
use std::{collections::HashSet, fmt::Write};
//[dev]:
// check two versions of encode & decode with regex and that now implemented for better perfomance
// https://users.rust-lang.org/t/encode-decode-uri/90017/15
//

/// This enumerates the various places where an error might occur parsing a
/// URI.
// [dev]
// make Context in separate folder andbe in same place with error.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Context {
    /// This is the fragment of the URI, such as `#baz` in
    /// `http://www.example.com/foo?bar#baz`.
    Fragment,

    /// This is the host name of the URI, such as `www.example.com` in
    /// `http://www.example.com/foo?bar#baz`.
    Host,

    /// This is the IPv4 portion of the IPv6 host name in the URI, such as
    /// `1.2.3.4` in `http://[::ffff:1.2.3.4]/foo?bar#baz`.
    Ipv4Address,

    /// This is the IPv6 host name in the URI, such as
    /// `::ffff:1.2.3.4` in `http://[::ffff:1.2.3.4]/foo?bar#baz`.
    Ipv6Address,

    /// This is the `IPvFuture` host name in the URI, such as
    /// `v7.aB` in `http://[v7.aB]/foo?bar#baz`.
    IpvFuture,

    /// This is the path of the URI, such as `/foo` in
    /// `http://www.example.com/foo?bar#baz`.
    Path,

    /// This is the query of the URI, such as `?bar` in
    /// `http://www.example.com/foo?bar#baz`.
    Query,

    /// This is the scheme of the URI, such as `http` in
    /// `http://www.example.com/foo?bar#baz`.
    Scheme,

    /// This is the scheme of the URI, such as `nobody` in
    /// `http://nobody@www.example.com/foo?bar#baz`.
    Userinfo,
}

impl std::fmt::Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Context::Fragment => write!(f, "fragment"),
            Context::Host => write!(f, "host"),
            Context::Ipv4Address => write!(f, "IPv4 address"),
            Context::Ipv6Address => write!(f, "IPv6 address"),
            Context::IpvFuture => write!(f, "IPvFuture"),
            Context::Path => write!(f, "path"),
            Context::Query => write!(f, "query"),
            Context::Scheme => write!(f, "scheme"),
            Context::Userinfo => write!(f, "user info"),
        }
    }
}

// https://datatracker.ietf.org/doc/html/rfc3986#section-2.4
pub struct PercentEncodedCharacterDecoder {
    decoded_character: u8,
    digits_left: usize,
}

impl PercentEncodedCharacterDecoder {
    pub fn new() -> Self {
        Self {
            decoded_character: 0,
            digits_left: 2,
        }
    }
    pub fn next(&mut self, c: char) -> Result<Option<u8>, ErrorKind> {
        self.shift_in_hex_digit(c)?;
        self.digits_left -= 1;
        if self.digits_left == 0 {
            let output = self.decoded_character;
            self.reset();
            Ok(Some(output))
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) {
        self.decoded_character = 0;
        self.digits_left = 2;
    }

    fn shift_in_hex_digit(&mut self, c: char) -> Result<(), ErrorKind> {
        self.decoded_character <<= 4;
        if let Some(ci) = c.to_digit(16) {
            self.decoded_character += u8::try_from(ci).unwrap();
        } else {
            self.reset();
            return Err(InvalidPercentEncoding);
        }
        Ok(())
    }
}

pub fn decode_element<T>(
    element: T,
    allowed_characters: &HashSet<char>,
    context: Context,
) -> Result<Vec<u8>, ErrorKind>
where
    T: AsRef<str>,
{
    let mut decoding_pec = false;
    let mut pec_decoder = PercentEncodedCharacterDecoder::new();
    element
        .as_ref()
        .chars()
        .filter_map(|c| {
            if decoding_pec {
                pec_decoder
                    .next(c)
                    .map_err(Into::into)
                    .transpose()
                    .map(|c| {
                        decoding_pec = false;
                        c
                    })
            } else if c == '%' {
                decoding_pec = true;
                None
            } else if allowed_characters.contains(&c) {
                Some(Ok(c as u8))
            } else {
                Some(Err(InvalidCharacter(context)))
            }
        })
        .collect()
}

pub fn encode_element(element: &[u8], allowed_characters: &HashSet<char>) -> String {
    let mut encoding = String::with_capacity(element.len());
    for ci in element {
        match char::try_from(*ci) {
            Ok(c) if allowed_characters.contains(&c) => encoding.push(c),
            _ => write!(encoding, "%{:02X}", ci).unwrap(),
        }
    }
    encoding
}
