use gpui::{AnyElement, App, IntoElement, Window};
use std::{fmt, ops::Range, rc::Rc};

pub(crate) type RouteFactory = Box<dyn Fn(RouteContext, &mut Window, &mut App) -> AnyElement>;
pub(crate) type LayoutFactory = Rc<dyn Fn(RouteContext, &mut Window, &mut App) -> AnyElement>;
pub(crate) type RouteGuard = Rc<dyn Fn(RouteContext, &mut Window, &mut App) -> GuardResult>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardResult {
    Allow,
    Deny,
    Redirect(Redirect),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Redirect {
    kind: RedirectKind,
    params: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RedirectKind {
    Path(String),
    Named(String),
}

impl Redirect {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            kind: RedirectKind::Named(name.into()),
            params: Vec::new(),
        }
    }

    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }
}

impl From<&str> for Redirect {
    fn from(path: &str) -> Self {
        Self {
            kind: RedirectKind::Path(path.to_owned()),
            params: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UrlError {
    UnknownName(String),
    MissingParameter { name: String, parameter: String },
}

impl fmt::Display for UrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownName(name) => write!(formatter, "unknown route name `{name}`"),
            Self::MissingParameter { name, parameter } => {
                write!(
                    formatter,
                    "missing parameter `{parameter}` for route `{name}`"
                )
            }
        }
    }
}

impl std::error::Error for UrlError {}

impl From<String> for Redirect {
    fn from(path: String) -> Self {
        Self {
            kind: RedirectKind::Path(path),
            params: Vec::new(),
        }
    }
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

#[derive(Debug, Eq, PartialEq)]
pub enum ParamError<E> {
    Missing(String),
    Parse(E),
}

impl<E: std::fmt::Display> std::fmt::Display for ParamError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(formatter, "missing route parameter `{name}`"),
            Self::Parse(error) => error.fmt(formatter),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ParamError<E> {}

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

pub trait GuardFactory<Kind>: 'static {
    #[doc(hidden)]
    fn run(&self, route: RouteContext, window: &mut Window, cx: &mut App) -> GuardResult;
}

impl<F> GuardFactory<WithoutContext> for F
where
    F: Fn() -> GuardResult + 'static,
{
    fn run(&self, _route: RouteContext, _window: &mut Window, _cx: &mut App) -> GuardResult {
        self()
    }
}

impl<F> GuardFactory<WithAppContext> for F
where
    F: Fn(&mut Window, &mut App) -> GuardResult + 'static,
{
    fn run(&self, _route: RouteContext, window: &mut Window, cx: &mut App) -> GuardResult {
        self(window, cx)
    }
}

impl<F> GuardFactory<WithRoute> for F
where
    F: Fn(RouteContext) -> GuardResult + 'static,
{
    fn run(&self, route: RouteContext, _window: &mut Window, _cx: &mut App) -> GuardResult {
        self(route)
    }
}

impl<F> GuardFactory<WithRouteContext> for F
where
    F: Fn(RouteContext, &mut Window, &mut App) -> GuardResult + 'static,
{
    fn run(&self, route: RouteContext, window: &mut Window, cx: &mut App) -> GuardResult {
        self(route, window, cx)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteContext {
    path: String,
    params: Vec<(String, String)>,
    query: Vec<(String, String)>,
}

impl RouteContext {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        lookup_pair(&self.params, name)
    }

    pub fn query(&self, name: &str) -> Option<&str> {
        lookup_pair(&self.query, name)
    }

    pub fn param_as<T: std::str::FromStr>(&self, name: &str) -> Result<T, ParamError<T::Err>> {
        parse_required(self.param(name), name)
    }

    pub fn query_as<T: std::str::FromStr>(&self, name: &str) -> Result<T, ParamError<T::Err>> {
        parse_required(self.query(name), name)
    }

    pub fn optional_param_as<T: std::str::FromStr>(&self, name: &str) -> Result<Option<T>, T::Err> {
        self.param(name).map(str::parse).transpose()
    }

    pub fn optional_query_as<T: std::str::FromStr>(&self, name: &str) -> Result<Option<T>, T::Err> {
        self.query(name).map(str::parse).transpose()
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
    pub(crate) layouts: Vec<LayoutFactory>,
    name: Option<String>,
    constraints: Vec<CompiledParamConstraint>,
    matchers: Vec<RouteMatcher>,
    is_catch_all: bool,
}

struct RouteMatcher {
    pattern: String,
    matcher: matchit::Router<()>,
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

pub(crate) fn normalize_location(location: &str) -> String {
    let (path, query) = parse_location(location);
    if query.is_empty() {
        path
    } else {
        format!("{path}?{}", serialize_query(&query))
    }
}

pub(crate) fn location_path(location: &str) -> String {
    parse_location(location).0
}

pub struct RouterConfig {
    pub(crate) routes: Vec<RouteEntry>,
    prefix: String,
    is_group: bool,
    last_registration: Option<LastRegistration>,
    layouts: Vec<LayoutFactory>,
    layout_declared: bool,
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
            layouts: Vec::new(),
            layout_declared: false,
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
            layouts: self.layouts.clone(),
            layout_declared: false,
        });

        let start = self.routes.len();
        for route in child.routes {
            self.insert_entry(
                route.path.clone(),
                route.path,
                route.factory,
                route.guards,
                route.layouts,
                route.constraints,
                route.name,
            );
        }
        self.last_registration = Some(LastRegistration::Group(start..self.routes.len()));
        self
    }

    pub fn layout<F, Kind>(mut self, factory: F) -> Self
    where
        F: PageFactory<Kind>,
    {
        if self.layout_declared {
            panic!("a group may only declare one layout");
        }
        if self.last_registration.is_some() {
            panic!("a layout must be declared before routes and groups");
        }
        self.layout_declared = true;
        self.layouts.push(Rc::new(move |route, window, cx| {
            factory.render(route, window, cx)
        }));
        self
    }

    pub fn guard<G, Kind>(mut self, guard: G) -> Self
    where
        G: GuardFactory<Kind>,
    {
        let guard: RouteGuard = Rc::new(move |route, window, cx| guard.run(route, window, cx));
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

    pub fn name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        let index = match self.last_registration.as_ref() {
            Some(LastRegistration::Route(index)) => *index,
            Some(LastRegistration::Group(_)) => {
                panic!("route names cannot be applied to a group")
            }
            None => panic!("a route name must follow a route"),
        };
        self.assign_name(index, name);
        self
    }

    pub fn url<K, V>(
        &self,
        name: &str,
        params: impl IntoIterator<Item = (K, V)>,
    ) -> Result<String, UrlError>
    where
        K: Into<String>,
        V: Into<String>,
    {
        let params = params
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        self.generate_named_url(name, params)
    }

    pub(crate) fn resolve_redirect(&self, redirect: &Redirect) -> Result<String, UrlError> {
        match &redirect.kind {
            RedirectKind::Path(path) => Ok(normalize_location(path)),
            RedirectKind::Named(name) => self.generate_named_url(name, redirect.params.clone()),
        }
    }

    fn generate_named_url(
        &self,
        name: &str,
        params: Vec<(String, String)>,
    ) -> Result<String, UrlError> {
        let Some(route) = self
            .routes
            .iter()
            .find(|route| route.name.as_deref() == Some(name))
        else {
            return Err(UrlError::UnknownName(name.to_owned()));
        };
        generate_url(&route.path, name, params)
    }

    fn assign_name(&mut self, index: usize, name: String) {
        if let Some(existing) = self
            .routes
            .iter()
            .find(|route| route.name.as_deref() == Some(name.as_str()))
        {
            panic!(
                "duplicate route name `{name}`: already registered by `{}`",
                existing.path
            );
        }
        self.routes[index].name = Some(name);
    }

    fn insert(&mut self, path: String, original_path: String, factory: RouteFactory) {
        self.insert_entry(
            path,
            original_path,
            factory,
            Vec::new(),
            self.layouts.clone(),
            Vec::new(),
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_entry(
        &mut self,
        path: String,
        original_path: String,
        factory: RouteFactory,
        guards: Vec<RouteGuard>,
        layouts: Vec<LayoutFactory>,
        constraints: Vec<CompiledParamConstraint>,
        name: Option<String>,
    ) {
        if self.routes.iter().any(|route| route.path == path) {
            panic!(
                "duplicate route `{path}`: `{original_path}` normalizes to an already registered path"
            );
        }
        let patterns = expand_optional_route(&path);
        for pattern in &patterns {
            if let Some(existing) = self
                .routes
                .iter()
                .flat_map(|route| &route.matchers)
                .find(|matcher| matcher.pattern == *pattern)
            {
                panic!(
                    "conflicting route pattern `{pattern}` generated by `{path}`; it is already registered by `{}`",
                    self.routes
                        .iter()
                        .find(|route| {
                            route
                                .matchers
                                .iter()
                                .any(|matcher| matcher.pattern == existing.pattern)
                        })
                        .unwrap()
                        .path
                );
            }
        }
        let matchers = patterns
            .into_iter()
            .map(|pattern| {
                let mut matcher = matchit::Router::new();
                matcher.insert(pattern.clone(), ()).unwrap_or_else(|error| {
                    panic!("invalid or conflicting route `{pattern}`: {error}")
                });
                RouteMatcher { pattern, matcher }
            })
            .collect();
        let is_catch_all = path
            .split('/')
            .any(|segment| segment.starts_with("{*") && segment.ends_with('}'));
        let index = self.routes.len();
        self.routes.push(RouteEntry {
            path,
            factory,
            guards,
            layouts,
            name: None,
            constraints,
            matchers,
            is_catch_all,
        });
        self.last_registration = Some(LastRegistration::Route(index));
        if let Some(name) = name {
            self.assign_name(index, name);
        }
    }

    pub(crate) fn match_route(&self, location: &str) -> Option<MatchedRoute> {
        let (path, query) = parse_location(location);
        self.routes
            .iter()
            .enumerate()
            .filter(|(_, route)| !route.is_catch_all)
            .find_map(|(index, route)| matched_route(index, route, &path, &query))
            .or_else(|| {
                self.routes
                    .iter()
                    .enumerate()
                    .filter(|(_, route)| route.is_catch_all)
                    .find_map(|(index, route)| matched_route(index, route, &path, &query))
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

fn matched_route(
    index: usize,
    route: &RouteEntry,
    path: &str,
    query: &[(String, String)],
) -> Option<MatchedRoute> {
    for matcher in &route.matchers {
        let Ok(matched) = matcher.matcher.at(path) else {
            continue;
        };
        if route.constraints.iter().any(|constraint| {
            matched
                .params
                .get(&constraint.parameter)
                .is_some_and(|value| !constraint.regex.is_match(value))
        }) {
            continue;
        }
        let params = matched
            .params
            .iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        return Some(MatchedRoute {
            index,
            context: RouteContext {
                path: path.to_owned(),
                params,
                query: query.to_vec(),
            },
        });
    }
    None
}

fn lookup_pair<'a>(pairs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn parse_required<T: std::str::FromStr>(
    value: Option<&str>,
    name: &str,
) -> Result<T, ParamError<T::Err>> {
    value
        .ok_or_else(|| ParamError::Missing(name.to_owned()))?
        .parse()
        .map_err(ParamError::Parse)
}

fn parse_location(location: &str) -> (String, Vec<(String, String)>) {
    let without_hash = location
        .split_once('#')
        .map(|(path, _)| path)
        .unwrap_or(location);
    let (path, query) = without_hash.split_once('?').unwrap_or((without_hash, ""));
    (normalize_path(path), parse_query(query))
}

fn parse_query(query: &str) -> Vec<(String, String)> {
    if query.is_empty() {
        return Vec::new();
    }
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

fn serialize_query(query: &[(String, String)]) -> String {
    query
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (from_hex(bytes[index + 1]), from_hex(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn percent_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn route_parameter_names(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter_map(|segment| {
        segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
            .map(|parameter| parameter.strip_prefix('*').unwrap_or(parameter))
            .map(|parameter| parameter.strip_suffix('?').unwrap_or(parameter))
    })
}

fn expand_optional_route(path: &str) -> Vec<String> {
    let segments: Vec<_> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    for (index, segment) in segments.iter().enumerate() {
        let Some(parameter) = segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix("?}"))
        else {
            continue;
        };
        let (parameter, is_catch_all) = parameter
            .strip_prefix('*')
            .map_or((parameter, false), |parameter| (parameter, true));
        if is_catch_all {
            panic!("catch-all parameter `{parameter}` cannot be optional");
        }
        if index + 1 != segments.len() {
            panic!("optional parameter `{parameter}` must be the final route segment");
        }

        let absent = normalize_path(&segments[..index].join("/"));
        let required = normalize_path(
            &segments
                .iter()
                .enumerate()
                .map(|(segment_index, segment)| {
                    if segment_index == index {
                        format!("{{{parameter}}}")
                    } else {
                        (*segment).to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join("/"),
        );
        return vec![absent, required];
    }
    vec![path.to_owned()]
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

fn generate_url(
    pattern: &str,
    name: &str,
    mut params: Vec<(String, String)>,
) -> Result<String, UrlError> {
    let mut segments = Vec::new();
    for segment in pattern.split('/').filter(|segment| !segment.is_empty()) {
        let Some(parameter) = segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
        else {
            segments.push(segment.to_owned());
            continue;
        };
        if let Some(parameter) = parameter.strip_prefix('*') {
            let value =
                take_param(&mut params, parameter).ok_or_else(|| UrlError::MissingParameter {
                    name: name.to_owned(),
                    parameter: parameter.to_owned(),
                })?;
            segments.extend(
                value
                    .split('/')
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(parameter) = parameter.strip_suffix('?') {
            if let Some(value) = take_param(&mut params, parameter) {
                segments.push(value);
            }
        } else {
            let value =
                take_param(&mut params, parameter).ok_or_else(|| UrlError::MissingParameter {
                    name: name.to_owned(),
                    parameter: parameter.to_owned(),
                })?;
            segments.push(value);
        }
    }
    let path = if segments.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", segments.join("/"))
    };
    Ok(if params.is_empty() {
        path
    } else {
        format!("{path}?{}", serialize_query(&params))
    })
}

fn take_param(params: &mut Vec<(String, String)>, name: &str) -> Option<String> {
    let index = params.iter().position(|(key, _)| key == name)?;
    Some(params.remove(index).1)
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
    use super::{
        GuardResult, ParamConstraint, ParamError, Redirect, RouterConfig, normalize_location,
        normalize_path,
    };

    #[test]
    fn attaches_a_layout_to_routes_in_the_current_group() {
        let config = RouterConfig::new()
            .layout(|| "shell")
            .route("/", || "home")
            .group("/dashboard", |routes| {
                routes
                    .layout(|| "dashboard")
                    .index(|| "dashboard home")
                    .route("settings", || "settings")
            })
            .route("/about", || "about");

        assert_eq!(config.routes[0].layouts.len(), 1);
        assert_eq!(config.routes[1].layouts.len(), 2);
        assert_eq!(config.routes[2].layouts.len(), 2);
        assert_eq!(config.routes[3].layouts.len(), 1);
    }

    #[test]
    fn nests_layouts_from_recursive_groups() {
        let config = RouterConfig::new().group("/dashboard", |routes| {
            routes.layout(|| "dashboard").group("users", |routes| {
                routes.layout(|| "users").route("{id}", || "user")
            })
        });

        assert_eq!(config.routes[0].layouts.len(), 2);
    }

    #[test]
    fn groups_without_a_layout_do_not_wrap_their_routes() {
        let config = RouterConfig::new()
            .layout(|| "shell")
            .group("/dashboard", |routes| routes.index(|| "dashboard"));

        assert_eq!(config.routes[0].layouts.len(), 1);
    }

    #[test]
    #[should_panic(expected = "a group may only declare one layout")]
    fn rejects_a_second_layout_on_the_same_group() {
        let _ = RouterConfig::new().layout(|| "first").layout(|| "second");
    }

    #[test]
    #[should_panic(expected = "a layout must be declared before routes and groups")]
    fn rejects_a_layout_after_a_route() {
        let _ = RouterConfig::new().route("/", || "home").layout(|| "shell");
    }

    #[test]
    fn generates_named_route_urls_and_puts_leftovers_in_the_query_string() {
        let config = RouterConfig::new()
            .route("/users/{id}", || "user")
            .name("users.show")
            .route("/login", || "login")
            .name("login");

        assert_eq!(
            config.url("users.show", [("id", "42")]).unwrap(),
            "/users/42"
        );
        assert_eq!(
            config
                .url("users.show", [("id", "42"), ("tab", "profile")])
                .unwrap(),
            "/users/42?tab=profile"
        );
        assert_eq!(
            config.url("login", Vec::<(&str, &str)>::new()).unwrap(),
            "/login"
        );
    }

    #[test]
    fn generates_optional_and_catch_all_named_route_urls() {
        let config = RouterConfig::new()
            .route("/users/{name?}", || "users")
            .name("users")
            .route("/files/{*path}", || "file")
            .name("files.show");

        assert_eq!(
            config.url("users", Vec::<(&str, &str)>::new()).unwrap(),
            "/users"
        );
        assert_eq!(
            config.url("users", [("name", "nils")]).unwrap(),
            "/users/nils"
        );
        assert_eq!(
            config
                .url("files.show", [("path", "documents/2026/report.pdf")])
                .unwrap(),
            "/files/documents/2026/report.pdf"
        );
    }

    #[test]
    fn generates_named_routes_registered_inside_groups() {
        let config = RouterConfig::new().group("/dashboard", |routes| {
            routes
                .route("users/{id}", || "user")
                .name("dashboard.users.show")
        });

        assert_eq!(
            config.url("dashboard.users.show", [("id", "7")]).unwrap(),
            "/dashboard/users/7"
        );
    }

    #[test]
    fn resolves_named_guard_redirects() {
        let config = RouterConfig::new()
            .route("/login", || "login")
            .name("login")
            .route("/users/{id}", || "user")
            .name("users.show");

        assert_eq!(
            config.resolve_redirect(&Redirect::named("login")).unwrap(),
            "/login"
        );
        assert_eq!(
            config
                .resolve_redirect(
                    &Redirect::named("users.show")
                        .param("id", "42")
                        .param("tab", "profile")
                )
                .unwrap(),
            "/users/42?tab=profile"
        );
        assert_eq!(
            config.resolve_redirect(&"/account".into()).unwrap(),
            "/account"
        );
    }

    #[test]
    fn returns_err_for_named_urls_with_missing_required_parameters() {
        let config = RouterConfig::new()
            .route("/users/{id}", || "user")
            .name("users.show");

        assert_eq!(
            config.url("users.show", Vec::<(&str, &str)>::new()),
            Err(crate::UrlError::MissingParameter {
                name: "users.show".into(),
                parameter: "id".into(),
            })
        );
    }

    #[test]
    fn returns_err_for_unknown_route_names() {
        assert_eq!(
            RouterConfig::new().url("missing", Vec::<(&str, &str)>::new()),
            Err(crate::UrlError::UnknownName("missing".into()))
        );
    }

    #[test]
    fn returns_err_for_named_redirects_with_unknown_or_incomplete_routes() {
        let config = RouterConfig::new()
            .route("/users/{id}", || "user")
            .name("users.show");

        assert_eq!(
            config.resolve_redirect(&Redirect::named("login")),
            Err(crate::UrlError::UnknownName("login".into()))
        );
        assert_eq!(
            config.resolve_redirect(&Redirect::named("users.show")),
            Err(crate::UrlError::MissingParameter {
                name: "users.show".into(),
                parameter: "id".into(),
            })
        );
    }

    #[test]
    #[should_panic(expected = "duplicate route name `users.show`")]
    fn rejects_duplicate_route_names() {
        let _ = RouterConfig::new()
            .route("/users/{id}", || "user")
            .name("users.show")
            .route("/profile/{id}", || "profile")
            .name("users.show");
    }

    #[test]
    #[should_panic(expected = "a route name must follow a route")]
    fn rejects_a_name_without_a_previous_route() {
        let _ = RouterConfig::new().name("home");
    }

    #[test]
    #[should_panic(expected = "route names cannot be applied to a group")]
    fn rejects_names_on_groups() {
        let _ = RouterConfig::new()
            .group("/users", |routes| routes.route("{id}", || "user"))
            .name("users");
    }

    #[test]
    fn matches_the_path_and_exposes_query_values() {
        let config = RouterConfig::new().route("/search", || "search");
        let matched = config.match_route("/search?q=rooter&sort=name").unwrap();

        assert_eq!(matched.context.path(), "/search");
        assert_eq!(matched.context.query("q"), Some("rooter"));
        assert_eq!(matched.context.query("sort"), Some("name"));
        assert_eq!(matched.context.query("missing"), None);
    }

    #[test]
    fn decodes_query_values_and_returns_the_first_duplicate() {
        let config = RouterConfig::new().route("/search", || "search");
        let matched = config
            .match_route("/search?q=hello%20world&q=second")
            .unwrap();

        assert_eq!(matched.context.query("q"), Some("hello world"));
    }

    #[test]
    fn parses_typed_query_values() {
        let config = RouterConfig::new().route("/items", || "items");
        let matched = config.match_route("/items?page=2").unwrap();

        assert_eq!(matched.context.query_as::<u64>("page"), Ok(2));
        assert_eq!(
            matched.context.query_as::<u64>("limit"),
            Err(ParamError::Missing("limit".to_owned()))
        );
        assert_eq!(matched.context.optional_query_as::<u64>("limit"), Ok(None));
    }

    #[test]
    fn parses_typed_route_parameters() {
        let config = RouterConfig::new().route("/users/{id}", || "user");
        let matched = config.match_route("/users/42").unwrap();

        assert_eq!(matched.context.param_as::<u64>("id"), Ok(42));
    }

    #[test]
    fn distinguishes_required_and_optional_missing_typed_parameters() {
        let config = RouterConfig::new().route("/users/{id?}", || "users");
        let matched = config.match_route("/users").unwrap();

        assert_eq!(
            matched.context.param_as::<u64>("id"),
            Err(ParamError::Missing("id".to_owned()))
        );
        assert_eq!(matched.context.optional_param_as::<u64>("id"), Ok(None));
    }

    #[test]
    fn returns_the_parse_error_for_an_invalid_typed_parameter() {
        let config = RouterConfig::new().route("/users/{id}", || "user");
        let matched = config.match_route("/users/not-a-number").unwrap();

        assert!(matches!(
            matched.context.param_as::<u64>("id"),
            Err(ParamError::Parse(_))
        ));
    }

    #[test]
    fn matches_optional_trailing_parameters_with_or_without_a_value() {
        let config = RouterConfig::new().route("/users/{name?}", || "users");

        let without_name = config.match_route("/users").unwrap();
        let with_name = config.match_route("/users/nils").unwrap();

        assert_eq!(config.routes.len(), 1);
        assert_eq!(without_name.context.param("name"), None);
        assert_eq!(with_name.context.param("name"), Some("nils"));
    }

    #[test]
    fn matches_optional_parameters_inside_recursive_groups() {
        let config = RouterConfig::new().group("/dashboard", |routes| {
            routes.group("users", |routes| routes.route("{name?}", || "users"))
        });

        assert_eq!(
            config.matched_path("/dashboard/users"),
            Some("/dashboard/users/{name?}")
        );
        assert_eq!(
            config.matched_path("/dashboard/users/nils"),
            Some("/dashboard/users/{name?}")
        );
    }

    #[test]
    fn only_checks_an_optional_parameter_constraint_when_present() {
        let config = RouterConfig::new()
            .route("/users/{id?}", || "users")
            .where_param("id", ParamConstraint::Number);

        assert!(config.match_route("/users").is_some());
        assert!(config.match_route("/users/42").is_some());
        assert!(config.match_route("/users/nils").is_none());
    }

    #[test]
    #[should_panic(expected = "optional parameter `name` must be the final route segment")]
    fn rejects_non_trailing_optional_parameters() {
        let _ = RouterConfig::new().route("/users/{name?}/settings", || "settings");
    }

    #[test]
    #[should_panic(expected = "catch-all parameter `path` cannot be optional")]
    fn rejects_optional_catch_all_parameters() {
        let _ = RouterConfig::new().route("/files/{*path?}", || "files");
    }

    #[test]
    #[should_panic(expected = "conflicting route pattern `/users`")]
    fn rejects_duplicates_created_by_an_optional_parameter() {
        let _ = RouterConfig::new()
            .route("/users/{name?}", || "optional")
            .route("/users", || "users");
    }

    #[test]
    #[should_panic(expected = "conflicting route pattern `/users/{name}`")]
    fn rejects_parameterized_duplicates_created_by_an_optional_parameter() {
        let _ = RouterConfig::new()
            .route("/users/{name}", || "user")
            .route("/users/{name?}", || "optional");
    }

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
            .guard(|| GuardResult::Allow)
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
            .guard(|| GuardResult::Allow)
            .route("/login", || "login");

        assert_eq!(config.routes[0].guards.len(), 1);
        assert_eq!(config.routes[1].guards.len(), 1);
        assert_eq!(config.routes[2].guards.len(), 1);
        assert!(config.routes[3].guards.is_empty());
    }

    #[test]
    fn accepts_route_aware_guards() {
        fn owns_user(route: super::RouteContext) -> GuardResult {
            if route.param("id") == Some("42") {
                GuardResult::Allow
            } else {
                GuardResult::Deny
            }
        }

        fn inspects_query(
            route: super::RouteContext,
            _window: &mut gpui::Window,
            _cx: &mut gpui::App,
        ) -> GuardResult {
            if route.query("tab") == Some("billing") {
                GuardResult::Allow
            } else {
                GuardResult::Redirect("/login".into())
            }
        }

        let config = RouterConfig::new()
            .route("/users/{id}", || "user")
            .guard(owns_user)
            .route("/account", || "account")
            .guard(inspects_query);

        assert_eq!(config.routes[0].guards.len(), 1);
        assert_eq!(config.routes[1].guards.len(), 1);
    }

    #[test]
    #[should_panic(expected = "a guard must follow a route or group")]
    fn rejects_a_guard_without_a_previous_route_or_group() {
        let _ = RouterConfig::new().guard(|| GuardResult::Allow);
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
    fn normalizes_locations_with_query_strings() {
        assert_eq!(normalize_location("/search?"), "/search");
        assert_eq!(
            normalize_location("//search///?q=rooter"),
            "/search?q=rooter"
        );
        assert_eq!(
            normalize_location("/search?q=rooter#ignored"),
            "/search?q=rooter"
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
