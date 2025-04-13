#[derive(Debug)]
pub enum Version {
    /// `HTTP/1.1`
    V1 = 1,
    /// `HTTP/2.0`
    V2 = 2,
    /// `HTTP/3.0`
    V3 = 3,
}
