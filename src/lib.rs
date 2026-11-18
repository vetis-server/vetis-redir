#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use ::http::header::LOCATION;
use cel::Context;
use compact_str::ToCompactString;
use http::{HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use vetis::{
    HookPhases, Request, Response, Str, VetisFutureResult, VetisHook, VetisResult,
    errors::{ConfigError, VetisError},
    hook::{Hook, HookConfig, HookPhase},
    make_bitflags,
    matcher::{Matcher, MatcherConfig},
};

#[cfg(test)]
mod tests;

/// Builder for creating `RedirHookConfig` instances.
#[derive(Deserialize)]
pub struct RedirHookConfigBuilder<'a> {
    matcher: Option<Box<dyn MatcherConfig>>,
    target: &'a str,
    code: &'a str,
}

impl<'a> RedirHookConfigBuilder<'a> {
    /// Allow set a matcher
    ///
    /// # Returns
    ///
    /// * `Self` - The builder.
    pub fn add_matcher<M>(mut self, matcher: M) -> Self
    where
        M: MatcherConfig + Send + Sync + 'static,
    {
        self.matcher = Some(Box::new(matcher));
        self
    }

    /// Allow set the target of the proxy path.
    ///
    /// # Returns
    ///
    /// * `Self` - The builder.
    pub fn target(mut self, target: &'a str) -> Self {
        self.target = target;
        self
    }

    /// Build the `ProxyHookConfig` with the configured settings.
    ///
    /// # Returns
    ///
    /// * `VetisResult<ProxyHookConfig>` - The `ProxyHookConfig` with the configured settings.
    pub fn build(self) -> VetisResult<RedirHookConfig> {
        if self
            .target
            .is_empty()
        {
            return Err(VetisError::Config(ConfigError::Hook(
                "Target cannot be empty".to_compact_string(),
            )));
        }

        Ok(RedirHookConfig {
            matcher: self
                .matcher
                .map_or(None, |val| Some(val)),
            target: self
                .target
                .to_compact_string(),
            code: "temporary".to_compact_string(),
        })
    }
}

/// Configuration for a proxy path.
#[derive(Clone, Serialize, Deserialize)]
pub struct RedirHookConfig {
    matcher: Option<Box<dyn MatcherConfig>>,
    target: Str,
    code: Str,
}

#[typetag::serde(name = "redir")]
impl HookConfig for RedirHookConfig {
    fn matcher(&self) -> Option<&Box<dyn MatcherConfig>> {
        self.matcher
            .as_ref()
    }

    fn boxed_clone(&self) -> Box<dyn HookConfig> {
        Box::new(self.clone())
    }

    fn boxed_hook(&self) -> Box<dyn VetisHook> {
        Box::new(RedirHook::new(self.clone()))
    }
}

impl RedirHookConfig {
    /// Creates a new `RedirHookConfigBuilder` with default settings.
    ///
    /// # Returns
    ///
    /// * `RedirHookConfigBuilder` - The builder.
    pub fn builder<'a>() -> RedirHookConfigBuilder<'a> {
        RedirHookConfigBuilder { matcher: None, target: "http://localhost:8080", code: "temporary" }
    }

    /// Returns the target of the proxy path.
    ///
    /// # Returns
    ///
    /// * `&str` - The target of the proxy path.
    pub fn target(&self) -> &str {
        &self.target
    }
}

/// Proxy path
pub struct RedirHook {
    config: RedirHookConfig,
    matcher: Option<Box<dyn Matcher>>,
}

unsafe impl Send for RedirHook {}
unsafe impl Sync for RedirHook {}

impl RedirHook {
    /// Create a new proxy path with provided configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The proxy path configuration
    ///
    /// # Returns
    ///
    /// * `RedirHook` - The proxy path
    pub fn new(config: RedirHookConfig) -> RedirHook {
        let matcher = if let Some(matcher_config) = config.matcher() {
            Some(matcher_config.boxed_matcher())
        } else {
            None
        };

        RedirHook { matcher, config: config }
    }
}

impl Hook for RedirHook {
    fn has_matches(&self, context: &Context) -> bool {
        if let Some(matcher) = &self.matcher { matcher.is_match(context) } else { true }
    }

    fn phases(&self) -> HookPhases {
        make_bitflags!(HookPhase::Rewrite)
    }

    fn matcher_mut(&mut self) -> Option<&mut Box<dyn Matcher>> {
        self.matcher
            .as_mut()
    }

    fn handle<'a>(
        &'a self,
        request: Request,
        _response: Option<Response>,
    ) -> VetisFutureResult<'a, (Request, Option<Response>)> {
        let future = async move {
            let status = match self
                .config
                .code
                .as_str()
            {
                "temporary" => StatusCode::TEMPORARY_REDIRECT,
                "permanent" => StatusCode::PERMANENT_REDIRECT,
                "html" => StatusCode::OK,
                _ => self
                    .config
                    .code
                    .parse::<StatusCode>()
                    .expect("Invalid status code"),
            };

            let header_value = HeaderValue::from_bytes(
                self.config
                    .target
                    .as_bytes(),
            )
            .map_err(|e| VetisError::hook(e.to_compact_string()))?;
            let response = Response::builder()
                .header(LOCATION, header_value)
                .status(status)
                .empty();
            Ok((request, Some(response)))
        };
        Box::pin(future)
    }
}
