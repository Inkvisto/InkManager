use http::uri::ipv4::validate_ipv4_address;

#[test]
fn correct_ipv4_addresses() {
    let addresses = [
        "0.0.0.0",
        "1.2.3.0",
        "1.2.3.4",
        "1.2.3.255",
        "1.2.255.4",
        "1.255.3.4",
        "255.2.3.4",
        "255.255.255.255",
    ];

    for address in addresses {
        assert!(validate_ipv4_address(address).is_ok());
    }
}
