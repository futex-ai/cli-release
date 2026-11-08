//! Release channel configuration cases.

use super::ReleaseChannel;

#[test]
fn parses_channels_with_stable_fallback() {
    for (value, expected) in [
        ("", ReleaseChannel::Stable),
        ("stable", ReleaseChannel::Stable),
        ("prerelease", ReleaseChannel::Prerelease),
        ("future-channel", ReleaseChannel::Stable),
    ] {
        assert_eq!(ReleaseChannel::from_config(value), expected, "{value}");
    }
}
