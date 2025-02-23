use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("Invalid status code")]
    InvalidStatusCode(String),
    #[error("Invalid scheme string")]
    InvalidScheme(String),
    // Method(method::InvalidMethod),
    // Uri(uri::InvalidUri),
    // UriParts(uri::InvalidUriParts),
    // HeaderName(header::InvalidHeaderName),
    // HeaderValue(header::InvalidHeaderValue),
    #[error("Invalid scheme length")]
    InvalidSchemeLength(usize),
}
