pub mod install;
pub mod start;
pub mod status;
pub mod stop;
pub mod uninstall;

pub(crate) const DNS_ADDR: &str = "127.0.0.1:9253";
pub(crate) const HTTPS_ADDR: &str = "0.0.0.0:443";
pub(crate) const RESOLVER_PATH: &str = "/etc/resolver/test";
pub(crate) const LAUNCHD_LABEL: &str = "com.dd-ferryman";
pub(crate) const PLIST_PATH: &str = "/Library/LaunchDaemons/com.dd-ferryman.plist";
