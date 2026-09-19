use gpui::{AnyElement, App, IntoElement, Window};
use std::{ops::Range, rc::Rc};

pub(crate) type RouteFactory = Box<dyn Fn(RouteContext, &mut Window, &mut App) -> AnyElement>;
pub(crate) type RouteGuard = Rc<dyn Fn(&mut Window, &mut App) -> GuardResult>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardResult {
    Allow,
    Deny,
    Redirect(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParamConstraint {
    Number,
    Alpha,
    AlphaNumeric,
    Uuid,
    Ulid,
    In(Vec<String>),
    Regex(String),
}

impl From<&str> for ParamConstraint {
    fn from(pattern: &str) -> Self {
        Self::Regex(pattern.to_owned())
    }
}

impl From<String> for ParamConstraint {
    fn from(pattern: String) -> Self {
        Self::Regex(pattern)
    }
}

#[doc(hidden)]
pub struct WithoutContext;
#[doc(hidden)]
pub struct WithAppContext;
#[doc(hidden)]
pub struct WithRoute;
#[doc(hidden)]
pub struct WithRouteContext;

pub trait PageFactory<Kind>: 'static {
    #[doc(hidden)]
    fn render(&self, route: RouteContext, window: &mut Window, cx: &mut App) -> AnyElement;
}

impl<F, E> PageFactory<WithoutContext> for F
where
    F: Fn() -> E + 'static,
    E: IntoElement,
{
    fn render(&self, _route: RouteContext, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self().into_any_element()
    }
}

impl<F, E> PageFactory<WithAppContext> for F
where
    F: Fn(&mut Window, &mut App) -> E + 'static,
    E: IntoElement,
{
    fn render(&self, _route: RouteContext, window: &mut Window, cx: &mut App) -> AnyElement {
        self(window, cx).into_any_element()
    }
}

impl<F, E> PageFactory<WithRoute> for F
where
    F: Fn(RouteContext) -> E + 'static,
    E: IntoElement,
{
    fn render(&self, route: RouteContext, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self(route).into_any_element()
    }
}

impl<F, E> PageFactory<WithRouteContext> for F
where
    F: Fn(RouteContext, &mut Window, &mut App) -> E + 'static,
    E: IntoElement,
{
    fn render(&self, route: RouteContext, window: &mut Window, cx: &mut App) -> AnyElement {
        self(route, window, cx).into_any_element()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteContext {
    path: String,
    params: Vec<(String, String)>,
}

impl RouteContext {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

pub(crate) struct MatchedRoute {
    pub(crate) index: usize,
    pub(crate) context: RouteContext,
}

pub(crate) struct RouteEntry {
    pub(crate) path: String,
    pub(crate) factory: RouteFactory,
    pub(crate) guards: Vec<RouteGuard>,
    constraints: Vec<CompiledParamConstraint>,
    matcher: matchit::Router<()>,
    is_catch_all: bool,
}

struct CompiledParamConstraint {
    parameter: String,
    regex: regex::Regex,
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
    last_registration: Option<LastRegistration>,
}

enum LastRegistration {
    Route(usize),
    Group(Range<usize>),
}

impl RouterConfig {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            prefix: "/".to_owned(),
            is_group: false,
            last_registration: None,
        }
    }

    pub fn route<F, Kind>(mut self, path: impl Into<String>, factory: F) -> Self
    where
        F: PageFactory<Kind>,
    {
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
            Box::new(move |route, window, cx| factory.render(route, window, cx)),
        );
        self
    }

    pub fn index<F, Kind>(mut self, factory: F) -> Self
    where
        F: PageFactory<Kind>,
    {
        if !self.is_group {
            panic!(
                "index routes can only be registered inside a group; use `route(\"/\", ...)` for the top-level route"
            );
        }
        let path = self.prefix.clone();
        self.insert(
            path.clone(),
            path,
            Box::new(move |route, window, cx| factory.render(route, window, cx)),
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
            last_registration: None,
        });

        let start = self.routes.len();
        for route in child.routes {
            self.insert_entry(
                route.path.clone(),
                route.path,
                route.factory,
                route.guards,
                route.constraints,
            );
        }
        self.last_registration = Some(LastRegistration::Group(start..self.routes.len()));
        self
    }

    pub fn guard(mut self, guard: impl Fn(&mut Window, &mut App) -> GuardResult + 'static) -> Self {
        let guard: RouteGuard = Rc::new(guard);
        match self.last_registration.as_ref() {
            Some(LastRegistration::Route(index)) => {
                self.routes[*index].guards.push(guard);
            }
            Some(LastRegistration::Group(range)) => {
                for route in &mut self.routes[range.clone()] {
                    route.guards.insert(0, guard.clone());
                }
            }
            None => panic!("a guard must follow a route or group"),
        }
        self
    }

    pub fn where_param(
        mut self,
        parameter: impl Into<String>,
        constraint: impl Into<ParamConstraint>,
    ) -> Self {
        let parameter = parameter.into();
        let index = match self.last_registration.as_ref() {
            Some(LastRegistration::Route(index)) => *index,
            Some(LastRegistration::Group(_)) => {
                panic!("parameter constraints cannot be applied to a group")
            }
            None => panic!("a parameter constraint must follow a route"),
        };
        let route = &mut self.routes[index];
        if !route_parameter_names(&route.path).any(|name| name == parameter) {
            panic!(
                "route `{}` has no parameter named `{parameter}`",
                route.path
            );
        }
        let regex = compile_constraint(constraint.into()).unwrap_or_else(|error| {
            panic!(
                "invalid constraint for parameter `{parameter}` on route `{}`: {error}",
                route.path
            )
        });
        route
            .constraints
            .push(CompiledParamConstraint { parameter, regex });
        self
    }

    fn insert(&mut self, path: String, original_path: String, factory: RouteFactory) {
        self.insert_entry(path, original_path, factory, Vec::new(), Vec::new());
    }

    fn insert_entry(
        &mut self,
        path: String,
        original_path: String,
        factory: RouteFactory,
        guards: Vec<RouteGuard>,
        constraints: Vec<CompiledParamConstraint>,
    ) {
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
        let index = self.routes.len();
        self.routes.push(RouteEntry {
            path,
            factory,
            guards,
            constraints,
            matcher,
            is_catch_all,
        });
        self.last_registration = Some(LastRegistration::Route(index));
    }

    pub(crate) fn match_route(&self, path: &str) -> Option<MatchedRoute> {
        self.routes
            .iter()
            .enumerate()
            .filter(|(_, route)| !route.is_catch_all)
            .find_map(|(index, route)| matched_route(index, route, path))
            .or_else(|| {
                self.routes
                    .iter()
                    .enumerate()
                    .filter(|(_, route)| route.is_catch_all)
                    .find_map(|(index, route)| matched_route(index, route, path))
            })
    }

    #[cfg(test)]
    pub(crate) fn match_index(&self, path: &str) -> Option<usize> {
        self.match_route(path).map(|matched| matched.index)
    }

    #[cfg(test)]
    fn matched_path(&self, path: &str) -> Option<&str> {
        self.match_index(path)
            .map(|index| self.routes[index].path.as_str())
    }
}

fn matched_route(index: usize, route: &RouteEntry, path: &str) -> Option<MatchedRoute> {
    let matched = route.matcher.at(path).ok()?;
    if route.constraints.iter().any(|constraint| {
        matched
            .params
            .get(&constraint.parameter)
            .is_none_or(|value| !constraint.regex.is_match(value))
    }) {
        return None;
    }
    let params = matched
        .params
        .iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
    Some(MatchedRoute {
        index,
        context: RouteContext {
            path: path.to_owned(),
            params,
        },
    })
}

fn route_parameter_names(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter_map(|segment| {
        segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
            .map(|parameter| parameter.strip_prefix('*').unwrap_or(parameter))
    })
}

fn compile_constraint(constraint: ParamConstraint) -> Result<regex::Regex, regex::Error> {
    let pattern = match constraint {
        ParamConstraint::Number => "[0-9]+".to_owned(),
        ParamConstraint::Alpha => "[a-zA-Z]+".to_owned(),
        ParamConstraint::AlphaNumeric => "[a-zA-Z0-9]+".to_owned(),
        ParamConstraint::Uuid => {
            "(?i:[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12})"
                .to_owned()
        }
        ParamConstraint::Ulid => "(?i:[0-7][0-9a-hjkmnp-tv-z]{25})".to_owned(),
        ParamConstraint::In(values) => values
            .iter()
            .map(|value| regex::escape(value))
            .collect::<Vec<_>>()
            .join("|"),
        ParamConstraint::Regex(pattern) => pattern,
    };
    regex::Regex::new(&format!("^(?:{pattern})$"))
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
    use super::{GuardResult, ParamConstraint, RouterConfig, normalize_path};

    #[test]
    fn supports_builtin_parameter_constraints() {
        let config = RouterConfig::new()
            .route("/numbers/{value}", || "number")
            .where_param("value", ParamConstraint::Number)
            .route("/alpha/{value}", || "alpha")
            .where_param("value", ParamConstraint::Alpha)
            .route("/alpha-numeric/{value}", || "alpha numeric")
            .where_param("value", ParamConstraint::AlphaNumeric)
            .route("/uuids/{value}", || "uuid")
            .where_param("value", ParamConstraint::Uuid)
            .route("/ulids/{value}", || "ulid")
            .where_param("value", ParamConstraint::Ulid)
            .route("/status/{value}", || "status")
            .where_param(
                "value",
                ParamConstraint::In(vec!["draft".to_owned(), "published".to_owned()]),
            );

        assert!(config.match_route("/numbers/42").is_some());
        assert!(config.match_route("/numbers/forty-two").is_none());
        assert!(config.match_route("/alpha/Rooter").is_some());
        assert!(config.match_route("/alpha/Rooter2").is_none());
        assert!(config.match_route("/alpha-numeric/Rooter2").is_some());
        assert!(config.match_route("/alpha-numeric/rooter-2").is_none());
        assert!(
            config
                .match_route("/uuids/550e8400-e29b-41d4-a716-446655440000")
                .is_some()
        );
        assert!(config.match_route("/uuids/not-a-uuid").is_none());
        assert!(
            config
                .match_route("/ulids/01ARZ3NDEKTSV4RRFFQ69G5FAV")
                .is_some()
        );
        assert!(config.match_route("/ulids/not-a-ulid").is_none());
        assert!(config.match_route("/status/draft").is_some());
        assert!(config.match_route("/status/archived").is_none());
    }

    #[test]
    fn supports_custom_regex_and_multiple_constraints() {
        let config = RouterConfig::new()
            .route("/posts/{year}/{slug}", || "post")
            .where_param("year", ParamConstraint::Number)
            .where_param("slug", "[a-z0-9-]+");

        assert!(config.match_route("/posts/2026/rooter-1").is_some());
        assert!(config.match_route("/posts/year/rooter-1").is_none());
        assert!(config.match_route("/posts/2026/Rooter").is_none());
    }

    #[test]
    fn constraints_preserve_declaration_order_and_catch_all_deferral() {
        let config = RouterConfig::new()
            .route("/files/{*path}", || "pdf")
            .where_param("path", r".+\.pdf")
            .route("/files/{name}", || "named")
            .where_param("name", "[a-z]+");

        assert_eq!(config.matched_path("/files/readme"), Some("/files/{name}"));
        assert_eq!(
            config.matched_path("/files/reports/2026.pdf"),
            Some("/files/{*path}")
        );
        assert_eq!(config.matched_path("/files/reports/2026.txt"), None);
    }

    #[test]
    #[should_panic(expected = "route `/users/{id}` has no parameter named `missing`")]
    fn rejects_constraints_for_unknown_parameters() {
        let _ = RouterConfig::new()
            .route("/users/{id}", || "user")
            .where_param("missing", ParamConstraint::Number);
    }

    #[test]
    #[should_panic(expected = "a parameter constraint must follow a route")]
    fn rejects_constraints_without_a_previous_route() {
        let _ = RouterConfig::new().where_param("id", ParamConstraint::Number);
    }

    #[test]
    #[should_panic(expected = "parameter constraints cannot be applied to a group")]
    fn rejects_constraints_on_groups() {
        let _ = RouterConfig::new()
            .group("/users", |routes| routes.route("{id}", || "user"))
            .where_param("id", ParamConstraint::Number);
    }

    #[test]
    #[should_panic(expected = "invalid constraint for parameter `slug` on route `/posts/{slug}`")]
    fn rejects_invalid_custom_regex_at_registration() {
        let _ = RouterConfig::new()
            .route("/posts/{slug}", || "post")
            .where_param("slug", "[");
    }

    #[test]
    fn extracts_required_route_parameters() {
        let config = RouterConfig::new().route("/users/{id}", || "user");

        let matched = config.match_route("/users/42").unwrap();

        assert_eq!(matched.index, 0);
        assert_eq!(matched.context.path(), "/users/42");
        assert_eq!(matched.context.param("id"), Some("42"));
        assert_eq!(matched.context.param("missing"), None);
    }

    #[test]
    fn extracts_the_full_catch_all_parameter() {
        let config = RouterConfig::new().route("/files/{*path}", || "file");

        let matched = config
            .match_route("/files/documents/2026/report.pdf")
            .unwrap();

        assert_eq!(
            matched.context.param("path"),
            Some("documents/2026/report.pdf")
        );
    }

    #[test]
    fn exposes_parameters_from_the_deferred_catch_all_match() {
        let config = RouterConfig::new()
            .route("/files/{*path}", || "fallback")
            .route("/files/{name}", || "file");

        let specific = config.match_route("/files/report.pdf").unwrap();
        let fallback = config
            .match_route("/files/documents/2026/report.pdf")
            .unwrap();

        assert_eq!(specific.index, 1);
        assert_eq!(specific.context.param("name"), Some("report.pdf"));
        assert_eq!(specific.context.param("path"), None);
        assert_eq!(fallback.index, 0);
        assert_eq!(
            fallback.context.param("path"),
            Some("documents/2026/report.pdf")
        );
    }

    #[test]
    fn accepts_route_aware_page_factories() {
        fn user_page(route: super::RouteContext) -> String {
            route.path().to_owned()
        }

        fn settings_page(
            route: super::RouteContext,
            _window: &mut gpui::Window,
            _cx: &mut gpui::App,
        ) -> String {
            route.path().to_owned()
        }

        let config = RouterConfig::new()
            .route("/users/{id}", user_page)
            .route("/settings/{section}", settings_page);

        assert_eq!(config.matched_path("/users/42"), Some("/users/{id}"));
        assert_eq!(
            config.matched_path("/settings/profile"),
            Some("/settings/{section}")
        );
    }

    #[test]
    fn attaches_a_guard_to_the_previous_route() {
        let config = RouterConfig::new()
            .route("/", || "home")
            .guard(|_, _| GuardResult::Allow)
            .route("/public", || "public");

        assert_eq!(config.routes[0].guards.len(), 1);
        assert!(config.routes[1].guards.is_empty());
    }

    #[test]
    fn attaches_a_guard_to_every_route_in_the_previous_group() {
        let config = RouterConfig::new()
            .group("/admin", |routes| {
                routes
                    .index(|| "dashboard")
                    .route("users", || "users")
                    .group("settings", |routes| routes.index(|| "settings"))
            })
            .guard(|_, _| GuardResult::Allow)
            .route("/login", || "login");

        assert_eq!(config.routes[0].guards.len(), 1);
        assert_eq!(config.routes[1].guards.len(), 1);
        assert_eq!(config.routes[2].guards.len(), 1);
        assert!(config.routes[3].guards.is_empty());
    }

    #[test]
    #[should_panic(expected = "a guard must follow a route or group")]
    fn rejects_a_guard_without_a_previous_route_or_group() {
        let _ = RouterConfig::new().guard(|_, _| GuardResult::Allow);
    }

    #[test]
    fn accepts_context_free_and_context_aware_page_factories() {
        fn settings(_window: &mut gpui::Window, _cx: &mut gpui::App) -> &'static str {
            "settings"
        }

        let config = RouterConfig::new()
            .route("/", || "home")
            .route("/settings", settings)
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
