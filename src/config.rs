use crate::github::GithubConfig;

#[derive(Clone)]
pub struct AppConfig {
    pub cookie_secure: bool,
    pub github: GithubConfig,
}