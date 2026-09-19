use gpui::{AnyElement, App, IntoElement, Window};

pub(crate) type RouteFactory = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;

pub trait PageFactory: 'static {
    #[doc(hidden)]
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement;
}

impl<F, E> PageFactory for F
where
    F: Fn() -> E + 'static,
    E: IntoElement,
{
    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self().into_any_element()
    }
}

pub struct ContextPageFactory<F>(F);

pub fn with_context<F, E>(factory: F) -> ContextPageFactory<F>
where
    F: Fn(&mut Window, &mut App) -> E + 'static,
    E: IntoElement,
{
    ContextPageFactory(factory)
}

impl<F, E> PageFactory for ContextPageFactory<F>
where
    F: Fn(&mut Window, &mut App) -> E + 'static,
    E: IntoElement,
{
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        (self.0)(window, cx).into_any_element()
    }
}

pub(crate) struct RouteEntry {
    pub(crate) path: String,
    pub(crate) factory: RouteFactory,
    matcher: matchit::Router<()>,
    is_catch_all: bool,
}

pub(crate) fn normalize_path(path: &str) -> String {
    let segments: Vec<_> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", segments.join("/"))
    }
}

pub struct RouterConfig {
    pub(crate) routes: Vec<RouteEntry>,
    prefix: String,
    is_group: bool,
}

impl RouterConfig {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            prefix: "/".to_owned(),
            is_group: false,
        }
    }

    pub fn route(mut self, path: impl Into<String>, factory: impl PageFactory) -> Self {
        let original_path = path.into();
        if self.is_group && original_path.starts_with('/') {
            panic!(
                "absolute child route `{original_path}` is not allowed inside group `{}`",
                self.prefix
            );
        }
        let path = if self.is_group {
            join_paths(&self.prefix, &original_path)
        } else {
            normalize_path(&original_path)
        };
        self.insert(
            path,
            original_path,
            Box::new(move |window, cx| factory.render(window, cx)),
        );
        self
    }

    pub fn index(mut self, factory: impl PageFactory) -> Self {
        if !self.is_group {
            panic!(
                "index routes can only be registered inside a group; use `route(\"/\", ...)` for the top-level route"
            );
        }
        let path = self.prefix.clone();
        self.insert(
            path.clone(),
            path,
            Box::new(move |window, cx| factory.render(window, cx)),
        );
        self
    }

    pub fn group(
        mut self,
        prefix: impl Into<String>,
        configure: impl FnOnce(RouterConfig) -> RouterConfig,
    ) -> Self {
        let original_prefix = prefix.into();
        if self.is_group && original_prefix.starts_with('/') {
            panic!(
                "absolute child group `{original_prefix}` is not allowed inside group `{}`",
                self.prefix
            );
        }
        let prefix = if self.is_group {
            join_paths(&self.prefix, &original_prefix)
        } else {
            normalize_path(&original_prefix)
        };
        let child = configure(RouterConfig {
            routes: Vec::new(),
            prefix,
            is_group: true,
        });

        for route in child.routes {
            self.insert(route.path.clone(), route.path, route.factory);
        }
        self
    }

    fn insert(&mut self, path: String, original_path: String, factory: RouteFactory) {
        if self.routes.iter().any(|route| route.path == path) {
            panic!(
                "duplicate route `{path}`: `{original_path}` normalizes to an already registered path"
            );
        }
        let mut matcher = matchit::Router::new();
        matcher
            .insert(path.clone(), ())
            .unwrap_or_else(|error| panic!("invalid or conflicting route `{path}`: {error}"));
        let is_catch_all = path
            .split('/')
            .any(|segment| segment.starts_with("{*") && segment.ends_with('}'));
        self.routes.push(RouteEntry {
            path,
            factory,
            matcher,
            is_catch_all,
        });
    }

    pub(crate) fn match_index(&self, path: &str) -> Option<usize> {
        self.routes
            .iter()
            .enumerate()
            .filter(|(_, route)| !route.is_catch_all)
            .find(|(_, route)| route.matcher.at(path).is_ok())
            .or_else(|| {
                self.routes
                    .iter()
                    .enumerate()
                    .filter(|(_, route)| route.is_catch_all)
                    .find(|(_, route)| route.matcher.at(path).is_ok())
            })
            .map(|(index, _)| index)
    }

    #[cfg(test)]
    fn matched_path(&self, path: &str) -> Option<&str> {
        self.match_index(path)
            .map(|index| self.routes[index].path.as_str())
    }
}

fn join_paths(prefix: &str, path: &str) -> String {
    normalize_path(&format!("{prefix}/{path}"))
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{RouterConfig, normalize_path, with_context};

    #[test]
    fn accepts_context_free_and_context_aware_page_factories() {
        let config = RouterConfig::new()
            .route("/", || "home")
            .route("/settings", with_context(|_window, _cx| "settings"))
            .group("/dashboard", |routes| routes.index(|| "dashboard"));

        assert_eq!(config.matched_path("/"), Some("/"));
        assert_eq!(config.matched_path("/settings"), Some("/settings"));
        assert_eq!(config.matched_path("/dashboard"), Some("/dashboard"));
    }

    #[test]
    fn normalizes_route_paths() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("about"), "/about");
        assert_eq!(
            normalize_path("//dashboard///settings/"),
            "/dashboard/settings"
        );
    }

    #[test]
    fn preserves_pattern_and_literal_segments() {
        assert_eq!(normalize_path("users/{id}/"), "/users/{id}");
        assert_eq!(normalize_path("users/{*rest}"), "/users/{*rest}");
        assert_eq!(normalize_path("./../settings"), "/./../settings");
    }

    #[test]
    #[should_panic(
        expected = "duplicate route `/about`: `/about/` normalizes to an already registered path"
    )]
    fn rejects_duplicate_normalized_routes() {
        let _ = RouterConfig::new()
            .route("about", || "first")
            .route("/about/", || "second");
    }

    #[test]
    fn flattens_recursive_groups_and_index_routes() {
        let config = RouterConfig::new()
            .route("/", || "home")
            .group("dashboard", |routes| {
                routes
                    .index(|| "dashboard")
                    .route("settings", || "settings")
                    .group("users", |routes| {
                        routes
                            .index(|| "users")
                            .route("{id}", || "user")
                            .route("{*rest}", || "users not found")
                    })
            })
            .route("{*rest}", || "not found");

        assert_eq!(config.matched_path("/"), Some("/"));
        assert_eq!(config.matched_path("/dashboard"), Some("/dashboard"));
        assert_eq!(
            config.matched_path("/dashboard/settings"),
            Some("/dashboard/settings")
        );
        assert_eq!(
            config.matched_path("/dashboard/users"),
            Some("/dashboard/users")
        );
        assert_eq!(
            config.matched_path("/dashboard/users/42"),
            Some("/dashboard/users/{id}")
        );
        assert_eq!(
            config.matched_path("/dashboard/users/missing/path"),
            Some("/dashboard/users/{*rest}")
        );
        assert_eq!(config.matched_path("/elsewhere"), Some("/{*rest}"));
    }

    #[test]
    #[should_panic(
        expected = "absolute child route `/settings` is not allowed inside group `/dashboard`"
    )]
    fn rejects_absolute_routes_inside_groups() {
        let _ = RouterConfig::new().group("dashboard", |routes| {
            routes.route("/settings", || "settings")
        });
    }

    #[test]
    #[should_panic(
        expected = "duplicate route `/dashboard`: `/dashboard` normalizes to an already registered path"
    )]
    fn rejects_group_index_collisions() {
        let _ = RouterConfig::new()
            .route("dashboard", || "first")
            .group("dashboard", |routes| routes.index(|| "second"));
    }

    #[test]
    #[should_panic(
        expected = "index routes can only be registered inside a group; use `route(\"/\", ...)` for the top-level route"
    )]
    fn rejects_top_level_index_routes() {
        let _ = RouterConfig::new().index(|| "home");
    }

    #[test]
    fn preserves_declaration_order_for_non_fallback_routes() {
        let config = RouterConfig::new()
            .route("/users/{id}", || "dynamic")
            .route("/users/new", || "static");

        assert_eq!(config.matched_path("/users/new"), Some("/users/{id}"));
    }

    #[test]
    fn defers_catch_all_routes_until_normal_routes_fail() {
        let config = RouterConfig::new()
            .route("/{*rest}", || "fallback")
            .route("/users/{id}", || "user");

        assert_eq!(config.matched_path("/users/42"), Some("/users/{id}"));
        assert_eq!(config.matched_path("/missing"), Some("/{*rest}"));
    }
}
