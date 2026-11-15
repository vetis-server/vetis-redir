#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
use cel::Context;
use compact_str::ToCompactString;
use serde::{Deserialize, Serialize};
use vetis::{
    Request, Response, VetisFutureResult, VetisHook, VetisResult,
    errors::{ConfigError, VetisError},
    hook::{Hook, HookConfig, HookPhase},
    matcher::{Matcher, MatcherConfig},
};

#[cfg(test)]
mod tests;

/// Builder for creating `RedirHookConfig` instances.
#[derive(Deserialize)]
pub struct RedirHookConfigBuilder {
    matcher: Option<Box<dyn MatcherConfig>>,
    target: String,
}

impl RedirHookConfigBuilder {
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
    pub fn target(mut self, target: &str) -> Self {
        self.target = target.to_string();
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
            target: self.target,
        })
    }
}

/// Configuration for a proxy path.
#[derive(Clone, Serialize, Deserialize)]
pub struct RedirHookConfig {
    matcher: Option<Box<dyn MatcherConfig>>,
    target: String,
    // TODO: Add custom proxy rules
    // TODO: Add support for custom headers
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
    pub fn builder() -> RedirHookConfigBuilder {
        RedirHookConfigBuilder { matcher: None, target: "http://localhost:8080".to_string() }
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
    _config: RedirHookConfig,
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

        RedirHook { matcher, _config: config }
    }
}

impl Hook for RedirHook {
    fn has_matches(&self, context: &Context) -> bool {
        if let Some(matcher) = &self.matcher { matcher.is_match(context) } else { true }
    }

    fn phase(&self) -> &HookPhase {
        &HookPhase::Rewrite
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
        let future = async move { Ok((request, Some(Response::builder().empty()))) };
        Box::pin(future)
    }
}
