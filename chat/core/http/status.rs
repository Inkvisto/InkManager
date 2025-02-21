//! HTTP status codes
//!
//! This module contains HTTP-status code related structs an errors. The main
//! type in this module is `StatusCode` which is not intended to be used through
//! this module but rather the `http::StatusCode` type.
//!
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StatusCode {
    // 1xx Informational
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    EarlyHints = 103,

    // 2xx Successful
    OK = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,
    IMUsed = 226,

    // 3xx Redirection
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    // 4xx Client Error
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    ContentTooLarge = 413,
    URITooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    MisdirectedRequest = 421,
    UnprocessableContent = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,

    // 5xx Server Error
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HTTPVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NotExtended = 510,
    NetworkAuthenticationRequired = 511,
}

#[allow(dead_code)]
impl StatusCode {
    #[inline]
    pub const fn as_u16(&self) -> u16 {
        *self as u16
    }

     /// Check if status is within 100-199.
     #[inline]  
     pub fn is_informational(&self) -> bool {
         200 > self.as_u16() && self.as_u16() >= 100
     }
 
     /// Check if status is within 200-299.
     #[inline]
     pub fn is_success(&self) -> bool {
         300 > self.as_u16() && self.as_u16() >= 200
     }
 
     /// Check if status is within 300-399.
     #[inline]
     pub fn is_redirection(&self) -> bool {
         400 > self.as_u16() && self.as_u16() >= 300
     }
 
     /// Check if status is within 400-499.
     #[inline]
     pub fn is_client_error(&self) -> bool {
         500 > self.as_u16() && self.as_u16() >= 400
     }
 
     /// Check if status is within 500-599.
     #[inline]
     pub fn is_server_error(&self) -> bool {
         600 > self.as_u16() && self.as_u16() >= 500
     }
}