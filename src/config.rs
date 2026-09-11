use crate::github::GithubConfig;
use anyhow::Context;
use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub cookie_secure: bool,
    pub redirect_frontend_url: String,
    pub reqwest_proxy:String,
    pub github: GithubConfig,
}
// #[derive(Debug,thiserror::Error)]
// pub enum ConfigError{
//     #[error("Environment variable {key} not found:{source}")]
//     VarError{
//         key: String,
//         #[source]
//         source:env::VarError
//     }
//
// }

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            cookie_secure: std::env::var("COOKIE_SECURE")
                .with_context(|| String::from("COOKIE_SECURE not found"))?
                .parse()?,
            reqwest_proxy: std::env::var("REQWEST_PROXY")
                .with_context(|| String::from("REQWEST_PROXY not found"))
                .unwrap_or(String::from("")),
            github: GithubConfig {
                client_id: std::env::var("CLIENT_ID")
                    .with_context(|| String::from("CLIENT_ID not found"))?,
                client_secret: std::env::var("CLIENT_SECRET")
                    .with_context(|| String::from("CLIENT_SECRET not found"))?,
                redirect_url: std::env::var("REDIRECT_URL")
                    .with_context(|| String::from("REDIRECT_URL not found"))?,
            },
            redirect_frontend_url: std::env::var("REDIRECT_FRONTEND_URL")
                .with_context(|| String::from("REDIRECT_FRONTEND_URL not found"))?,
        })
    }
}
