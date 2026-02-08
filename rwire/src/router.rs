//! Client-side routing for rwire single-page applications.
//!
//! Provides URL-based navigation without full page reloads. Registered views
//! are automatically tree-shaken at startup so their element types, style
//! tokens, and events are included in the capsule.
//!
//! # Example
//!
//! ```ignore
//! use rwire::router::{Router, Link};
//!
//! Server::bind("0.0.0.0:9000")?
//!     .root(root)
//!     .routes(
//!         Router::new()
//!             .page("/", |_| build_landing())
//!             .page("/users/:id", |p| build_user(p))
//!     )
//!     .run()
//!     .await
//! ```

use crate::builder::ElementBuilder;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Route Parameters
// ============================================================================

/// Extracted parameters from a matched route pattern.
///
/// Named params (`:id`) are accessible via `get()`, wildcard matches (`*`)
/// via `wildcard()`.
#[derive(Clone, Debug, Default)]
pub struct RouteParams {
    params: HashMap<String, String>,
    wildcard: Option<String>,
}

impl RouteParams {
    /// Create empty params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a named parameter by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// Get the wildcard match (everything after `*`).
    pub fn wildcard(&self) -> Option<&str> {
        self.wildcard.as_deref()
    }

    /// Insert a named parameter.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.insert(key.into(), value.into());
    }

    /// Set the wildcard value.
    pub fn set_wildcard(&mut self, value: impl Into<String>) {
        self.wildcard = Some(value.into());
    }
}

// ============================================================================
// Route Pattern
// ============================================================================

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
    pub fn matches(&self, path: &str) -> Option<RouteParams> {
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        let has_wildcard = self
            .segments
            .iter()
            .any(|s| matches!(s, PatternSegment::Wildcard));

        if !has_wildcard && path_segments.len() != self.segments.len() {
            return None;
        }

        let mut params = RouteParams::new();

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
                    let rest = path_segments[i..].join("/");
                    params.set_wildcard(rest);
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

// ============================================================================
// View Function Type
// ============================================================================

/// A view function that builds page content from route parameters.
pub type ViewFn = Arc<dyn Fn(&RouteParams) -> ElementBuilder + Send + Sync>;

// ============================================================================
// Route
// ============================================================================

/// A single route definition pairing a pattern with a view function.
pub struct Route {
    pattern: RoutePattern,
    view: ViewFn,
}

impl Route {
    /// Check if this route matches a path.
    pub fn matches(&self, path: &str) -> Option<RouteParams> {
        self.pattern.matches(path)
    }

    /// Build the element for this route with the given params.
    pub fn build(&self, params: &RouteParams) -> ElementBuilder {
        (self.view)(params)
    }

    /// Get the pattern string.
    pub fn pattern_str(&self) -> &str {
        self.pattern.pattern()
    }
}

// ============================================================================
// Router
// ============================================================================

/// Router for managing client-side navigation and view tree-shaking.
///
/// Register page views with `.page()`. At server startup, all views are
/// called with default params to discover their element types, style tokens,
/// and events for inclusion in the capsule (automatic tree-shaking).
#[derive(Default)]
pub struct Router {
    routes: Vec<Route>,
    not_found: Option<ViewFn>,
}

impl Router {
    /// Create a new router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a page route with a view function.
    ///
    /// The view function receives `&RouteParams` and returns an `ElementBuilder`.
    /// It is called at startup with empty params for tree-shaking, and at
    /// runtime when the route is matched.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Router::new()
    ///     .page("/", |_| build_home())
    ///     .page("/users/:id", |params| {
    ///         let id = params.get("id").unwrap_or("0");
    ///         build_user_page(id)
    ///     })
    ///     .page("/docs/*", |params| {
    ///         let path = params.wildcard().unwrap_or("");
    ///         build_doc_page(path)
    ///     })
    /// ```
    pub fn page<F>(mut self, pattern: &str, view: F) -> Self
    where
        F: Fn(&RouteParams) -> ElementBuilder + Send + Sync + 'static,
    {
        self.routes.push(Route {
            pattern: RoutePattern::new(pattern),
            view: Arc::new(view),
        });
        self
    }

    /// Set the 404 not found view.
    pub fn not_found<F>(mut self, view: F) -> Self
    where
        F: Fn(&RouteParams) -> ElementBuilder + Send + Sync + 'static,
    {
        self.not_found = Some(Arc::new(view));
        self
    }

    /// Match a path and return the matched route and extracted params.
    pub fn match_path(&self, path: &str) -> Option<(&Route, RouteParams)> {
        for route in &self.routes {
            if let Some(params) = route.matches(path) {
                return Some((route, params));
            }
        }
        None
    }

    /// Build the element for a specific path.
    pub fn build_for_path(&self, path: &str) -> ElementBuilder {
        if let Some((route, params)) = self.match_path(path) {
            route.build(&params)
        } else if let Some(ref not_found) = self.not_found {
            not_found(&RouteParams::new())
        } else {
            use crate::builder::el;
            use crate::protocol::El;
            el(El::Div).text("404 - Not Found")
        }
    }

    /// Call all registered view functions with default params and return
    /// the resulting element trees. Used at startup for tree-shaking.
    pub fn tree_shake_views(&self) -> Vec<ElementBuilder> {
        let default_params = RouteParams::new();
        let mut trees: Vec<ElementBuilder> = self
            .routes
            .iter()
            .map(|r| (r.view)(&default_params))
            .collect();

        if let Some(ref not_found) = self.not_found {
            trees.push(not_found(&default_params));
        }

        trees
    }

    /// Get all route patterns (for diagnostics).
    pub fn patterns(&self) -> Vec<&str> {
        self.routes.iter().map(|r| r.pattern.pattern()).collect()
    }

    /// Get number of registered routes.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check if router has no routes.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

// ============================================================================
// Link Helper
// ============================================================================

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
        assert_eq!(params.get("id"), Some("123"));

        assert!(pattern.matches("/users").is_none());
        assert!(pattern.matches("/posts/123").is_none());
    }

    #[test]
    fn test_pattern_multiple_params() {
        let pattern = RoutePattern::new("/users/:user_id/posts/:post_id");

        let result = pattern.matches("/users/42/posts/99");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("user_id"), Some("42"));
        assert_eq!(params.get("post_id"), Some("99"));
    }

    #[test]
    fn test_pattern_wildcard() {
        let pattern = RoutePattern::new("/docs/*");
        let result = pattern.matches("/docs/getting-started/install");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.wildcard(), Some("getting-started/install"));

        // Wildcard with empty rest
        let result = pattern.matches("/docs/");
        assert!(result.is_some());
        assert_eq!(result.unwrap().wildcard(), Some(""));
    }

    #[test]
    fn test_pattern_root() {
        let pattern = RoutePattern::new("/");
        assert!(pattern.matches("/").is_some());
        assert!(pattern.matches("/users").is_none());
    }

    #[test]
    fn test_route_params() {
        let mut params = RouteParams::new();
        params.insert("id", "42");
        params.set_wildcard("rest/of/path");

        assert_eq!(params.get("id"), Some("42"));
        assert_eq!(params.get("missing"), None);
        assert_eq!(params.wildcard(), Some("rest/of/path"));
    }

    #[test]
    fn test_router_page() {
        let router = Router::new()
            .page("/", |_| {
                use crate::builder::el;
                use crate::protocol::El;
                el(El::Div).text("Home")
            })
            .page("/users", |_| {
                use crate::builder::el;
                use crate::protocol::El;
                el(El::Div).text("Users")
            });

        assert!(router.match_path("/").is_some());
        assert!(router.match_path("/users").is_some());
        assert!(router.match_path("/posts").is_none());
    }

    #[test]
    fn test_router_tree_shake() {
        let router = Router::new()
            .page("/", |_| {
                use crate::builder::el;
                use crate::protocol::El;
                el(El::H1).text("Home")
            })
            .page("/about", |_| {
                use crate::builder::el;
                use crate::protocol::El;
                el(El::H2).text("About")
            });

        let trees = router.tree_shake_views();
        assert_eq!(trees.len(), 2);
    }
}
