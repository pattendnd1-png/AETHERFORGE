#![forbid(unsafe_code)]
//! Tracker, cookie, and browser privacy policy contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackingProtectionLevel {
    Off,
    Standard,
    Strict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThirdPartyCookiePolicy {
    Allow,
    Partition,
    Block,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrivacyPolicy {
    pub tracking: TrackingProtectionLevel,
    pub third_party_cookies: ThirdPartyCookiePolicy,
    pub upgrade_https: bool,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            tracking: TrackingProtectionLevel::Standard,
            third_party_cookies: ThirdPartyCookiePolicy::Partition,
            upgrade_https: true,
        }
    }
}
