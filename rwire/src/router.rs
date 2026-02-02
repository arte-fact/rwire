//! Client-side routing for rwire single-page applications.
//!
//! Provides URL-based navigation without full page reloads.
//!
//! # Example
//!
//! ```ignore
//! use rwire::router::{Router, Route};
//!
//! fn build_app() -> ElementBuilder {
//!     Router::new()
//!         .route("/", home_page)
//!         .route("/users", users_list)
//!         .route("/users/:id", user_detail)
//!         .not_found(not_found_page)
//!         .build()
//! }
//! ```

use crate::builder::ElementBuilder;
use std::collections::HashMap;

/// A route pattern that can match URLs.
#[derive(Clone, Debug)]
pub struct RoutePattern {
    /// The original pattern string (e.g., "/users/:id").
    pattern: String,
    /// Segments of the pattern.
    segments: Vec<PatternSegment>,
}

#[derive(Clone, Debug)]
enum PatternSegment {
    /// Literal segment that must match exactly.
    Literal(String),
    /// Parameter segment that captures a value (e.g., ":id").
    Param(String),
    /// Wildcard that matches anything remaining.
    Wildcard,
}

impl RoutePattern {
    /// Parse a route pattern string.
    pub fn new(pattern: &str) -> Self {
        let segments = pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                if let Some(param_name) = s.strip_prefix(':') {
                    PatternSegment::Param(param_name.to_string())
                } else if s == "*" {
                    PatternSegment::Wildcard
                } else {
                    PatternSegment::Literal(s.to_string())
                }
            })
            .collect();

        Self {
            pattern: pattern.to_string(),
            segments,
        }
    }

    /// Check if a path matches this pattern, returning captured parameters.
    pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // Check if we have a wildcard - it matches anything
        let has_wildcard = self
            .segments
            .iter()
            .any(|s| matches!(s, PatternSegment::Wildcard));

        if !has_wildcard && path_segments.len() != self.segments.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (i, segment) in self.segments.iter().enumerate() {
            match segment {
                PatternSegment::Literal(expected) => {
                    if path_segments.get(i) != Some(&expected.as_str()) {
                        return None;
                    }
                }
                PatternSegment::Param(name) => {
                    if let Some(&value) = path_segments.get(i) {
                        params.insert(name.clone(), value.to_string());
                    } else {
                        return None;
                    }
                }
                PatternSegment::Wildcard => {
                    // Wildcard matches the rest
                    let rest = path_segments[i..].join("/");
                    params.insert("*".to_string(), rest);
                    break;
                }
            }
        }

        Some(params)
    }

    /// Get the pattern string.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

/// A single route definition.
#[derive(Clone)]
pub struct Route {
    pattern: RoutePattern,
    builder: fn() -> ElementBuilder,
}

impl Route {
    /// Create a new route.
    pub fn new(pattern: &str, builder: fn() -> ElementBuilder) -> Self {
        Self {
            pattern: RoutePattern::new(pattern),
            builder,
        }
    }

    /// Check if this route matches a path.
    pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
        self.pattern.matches(path)
    }

    /// Build the element for this route.
    pub fn build(&self) -> ElementBuilder {
        (self.builder)()
    }
}

/// Router for managing client-side navigation.
#[derive(Clone, Default)]
pub struct Router {
    routes: Vec<Route>,
    not_found: Option<fn() -> ElementBuilder>,
}

impl Router {
    /// Create a new router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a route to the router.
    pub fn route(mut self, pattern: &str, builder: fn() -> ElementBuilder) -> Self {
        self.routes.push(Route::new(pattern, builder));
        self
    }

    /// Set the 404 not found handler.
    pub fn not_found(mut self, builder: fn() -> ElementBuilder) -> Self {
        self.not_found = Some(builder);
        self
    }

    /// Match a path and return the route and parameters.
    pub fn match_path(&self, path: &str) -> Option<(&Route, HashMap<String, String>)> {
        for route in &self.routes {
            if let Some(params) = route.matches(path) {
                return Some((route, params));
            }
        }
        None
    }

    /// Build the element for the current path.
    ///
    /// In a real implementation, this would get the path from the browser.
    /// For now, it defaults to "/" or uses the first route.
    pub fn build(self) -> ElementBuilder {
        self.build_for_path("/")
    }

    /// Build the element for a specific path.
    pub fn build_for_path(&self, path: &str) -> ElementBuilder {
        if let Some((route, _params)) = self.match_path(path) {
            route.build()
        } else if let Some(not_found) = self.not_found {
            not_found()
        } else {
            // Default fallback
            use crate::builder::el;
            use crate::protocol::El;
            el(El::Div).text("404 - Not Found")
        }
    }

    /// Get all route patterns (for client-side routing setup).
    pub fn patterns(&self) -> Vec<&str> {
        self.routes.iter().map(|r| r.pattern.pattern()).collect()
    }
}

/// Helper for creating navigation links.
pub struct Link;

impl Link {
    /// Create a navigation link element.
    ///
    /// The link will use client-side routing instead of full page navigation.
    pub fn to(href: &str, text: &str) -> ElementBuilder {
        use crate::builder::el;
        use crate::protocol::El;

        el(El::A)
            .attr("href", href)
            .attr("data-route", "")
            .text(text)
    }

    /// Create a navigation link with custom content.
    pub fn to_with_content(href: &str, content: ElementBuilder) -> ElementBuilder {
        use crate::builder::el;
        use crate::protocol::El;

        el(El::A)
            .attr("href", href)
            .attr("data-route", "")
            .append([content])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_literal() {
        let pattern = RoutePattern::new("/users");
        assert!(pattern.matches("/users").is_some());
        assert!(pattern.matches("/posts").is_none());
        assert!(pattern.matches("/users/123").is_none());
    }

    #[test]
    fn test_pattern_param() {
        let pattern = RoutePattern::new("/users/:id");

        let result = pattern.matches("/users/123");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        assert!(pattern.matches("/users").is_none());
        assert!(pattern.matches("/posts/123").is_none());
    }

    #[test]
    fn test_pattern_multiple_params() {
        let pattern = RoutePattern::new("/users/:user_id/posts/:post_id");

        let result = pattern.matches("/users/42/posts/99");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("user_id"), Some(&"42".to_string()));
        assert_eq!(params.get("post_id"), Some(&"99".to_string()));
    }

    #[test]
    fn test_pattern_root() {
        let pattern = RoutePattern::new("/");
        assert!(pattern.matches("/").is_some());
        assert!(pattern.matches("/users").is_none());
    }

    #[test]
    fn test_router_match() {
        fn home() -> ElementBuilder {
            use crate::builder::el;
            use crate::protocol::El;
            el(El::Div).text("Home")
        }

        fn users() -> ElementBuilder {
            use crate::builder::el;
            use crate::protocol::El;
            el(El::Div).text("Users")
        }

        let router = Router::new().route("/", home).route("/users", users);

        assert!(router.match_path("/").is_some());
        assert!(router.match_path("/users").is_some());
        assert!(router.match_path("/posts").is_none());
    }
}
