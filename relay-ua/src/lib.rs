//! User agent parser with built-in rules.
//!
//! # Test Performance
//!
//! Adding user agent parsing to your module will incur a latency penalty on first use. Because of
//! this, integration tests could fail. To fix this, you will need to add a timeout to your
//! consumer.

use std::borrow::Cow;
use std::sync::LazyLock;

use ua_parser::{Extractor, Regexes};

/// The family and version of a user agent client.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UserAgent<'a> {
    /// The client family, for example `Chrome`, or `Other` if unknown.
    pub family: Cow<'a, str>,
    /// The major version.
    pub major: Option<Cow<'a, str>>,
    /// The minor version.
    pub minor: Option<Cow<'a, str>>,
    /// The patch version.
    pub patch: Option<Cow<'a, str>>,
}

impl Default for UserAgent<'_> {
    fn default() -> Self {
        Self {
            family: Cow::Borrowed("Other"),
            major: None,
            minor: None,
            patch: None,
        }
    }
}

/// The family, brand, and model of a device.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Device<'a> {
    /// The device family, or `Other` if unknown.
    pub family: Cow<'a, str>,
    /// The brand of the device.
    pub brand: Option<Cow<'a, str>>,
    /// The model of the device.
    pub model: Option<Cow<'a, str>>,
}

impl Default for Device<'_> {
    fn default() -> Self {
        Self {
            family: Cow::Borrowed("Other"),
            brand: None,
            model: None,
        }
    }
}

/// The family and version of an operating system.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OS<'a> {
    /// The operating system family, or `Other` if unknown.
    pub family: Cow<'a, str>,
    /// The major version.
    pub major: Option<Cow<'a, str>>,
    /// The minor version.
    pub minor: Option<Cow<'a, str>>,
    /// The patch version.
    pub patch: Option<Cow<'a, str>>,
    /// The fourth version component.
    pub patch_minor: Option<Cow<'a, str>>,
}

impl Default for OS<'_> {
    fn default() -> Self {
        Self {
            family: Cow::Borrowed("Other"),
            major: None,
            minor: None,
            patch: None,
            patch_minor: None,
        }
    }
}

/// The global user agent extractor, configured with the uap-core rules.
///
/// Uses `ua-parser` from the uap-rust project, which prefilters the rules: one pass over the input
/// finds the literal strings that occur in it, and only rules whose required literals are all
/// present are run. The previous parser tried every rule in turn, which cost about 280µs per user
/// agent compared to about 15µs here, with identical results on uap-core's test suite.
static UA_PARSER: LazyLock<Extractor<'static>> = LazyLock::new(|| {
    let ua_regexes: &'static [u8] = include_bytes!("../uap-core/regexes.yaml");
    let regexes: Regexes<'static> = serde_yaml::from_slice(ua_regexes)
        .expect("Could not parse user agent rules. You are probably using a bad build of relay.");
    Extractor::try_from(regexes)
        .expect("Could not create UserAgent. You are probably using a bad build of relay.")
});

/// Initializes the user agent parser.
///
/// This loads and compiles user agent patterns, which takes a few seconds to complete. The user
/// agent parser initializes on-demand when using one of the parse methods. This function forces
/// initialization at a convenient point without introducing unwanted delays.
pub fn init_parser() {
    LazyLock::force(&UA_PARSER);
}

/// Returns the family and version of a user agent client.
///
/// Defaults to an empty user agent.
pub fn parse_user_agent(user_agent: &str) -> UserAgent<'_> {
    match UA_PARSER.ua.extract(user_agent) {
        Some(ua) => UserAgent {
            family: ua.family,
            major: ua.major.map(Cow::Borrowed),
            minor: ua.minor.map(Cow::Borrowed),
            patch: ua.patch.map(Cow::Borrowed),
        },
        None => UserAgent::default(),
    }
}

/// Returns the family, brand, and model of the device of the requesting client.
///
/// Defaults to an empty device.
pub fn parse_device(user_agent: &str) -> Device<'_> {
    match UA_PARSER.dev.extract(user_agent) {
        Some(device) => Device {
            family: device.device,
            brand: device.brand,
            model: device.model,
        },
        None => Device::default(),
    }
}

/// Returns the family and version of the operating system of the requesting client.
///
/// Defaults to an empty operating system.
pub fn parse_os(user_agent: &str) -> OS<'_> {
    match UA_PARSER.os.extract(user_agent) {
        Some(os) => OS {
            family: os.os,
            major: os.major,
            minor: os.minor,
            patch: os.patch,
            patch_minor: os.patch_minor,
        },
        None => OS::default(),
    }
}

#[cfg(test)]
mod tests {
    use serde_yaml::Value;

    use super::*;

    /// Test cases shipped with uap-core, each a user agent and its expected values.
    fn cases(file: &str) -> Vec<Value> {
        let path = format!("{}/uap-core/tests/{file}", env!("CARGO_MANIFEST_DIR"));
        let yaml: Value = serde_yaml::from_slice(&std::fs::read(path).unwrap()).unwrap();
        yaml["test_cases"].as_sequence().unwrap().clone()
    }

    fn field(case: &Value, key: &str) -> Option<String> {
        case[key].as_str().map(str::to_owned)
    }

    fn owned(value: Option<Cow<'_, str>>) -> Option<String> {
        value.map(Cow::into_owned)
    }

    #[test]
    fn test_uap_core_user_agents() {
        for case in cases("test_ua.yaml") {
            let input = case["user_agent_string"].as_str().unwrap();
            let ua = parse_user_agent(input);
            assert_eq!(
                (
                    ua.family.into_owned(),
                    owned(ua.major),
                    owned(ua.minor),
                    owned(ua.patch)
                ),
                (
                    field(&case, "family").unwrap(),
                    field(&case, "major"),
                    field(&case, "minor"),
                    field(&case, "patch"),
                ),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn test_uap_core_operating_systems() {
        for case in cases("test_os.yaml") {
            let input = case["user_agent_string"].as_str().unwrap();
            let os = parse_os(input);
            assert_eq!(
                (
                    os.family.into_owned(),
                    owned(os.major),
                    owned(os.minor),
                    owned(os.patch),
                    owned(os.patch_minor),
                ),
                (
                    field(&case, "family").unwrap(),
                    field(&case, "major"),
                    field(&case, "minor"),
                    field(&case, "patch"),
                    field(&case, "patch_minor"),
                ),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn test_uap_core_devices() {
        for case in cases("test_device.yaml") {
            let input = case["user_agent_string"].as_str().unwrap();
            let device = parse_device(input);
            assert_eq!(
                (
                    device.family.into_owned(),
                    owned(device.brand),
                    owned(device.model)
                ),
                (
                    field(&case, "family").unwrap(),
                    field(&case, "brand"),
                    field(&case, "model"),
                ),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn test_unknown_user_agent() {
        assert_eq!(parse_user_agent("not a browser"), UserAgent::default());
        assert_eq!(parse_device(""), Device::default());
        assert_eq!(parse_os("curl/8.5.0"), OS::default());
    }
}
