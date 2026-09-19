use crate::{GuardResult, RouterConfig, UrlError, config::normalize_location, outlet::push_outlet};
use gpui::{
    App, AppContext, Context, Empty, Entity, EventEmitter, Global, IntoElement, Render,
    SharedString, WeakEntity, Window, WindowId,
};
use std::collections::HashMap;

/// How the current location was reached.
///
/// Extra variants may be added in 0.x without a major version bump.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum NavigationKind {
    Push,
    Replace,
    Back,
    Forward,
}

/// Emitted after the location changes.
///
/// Extra fields may be added in 0.x without a major version bump.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct NavigationEvent {
    pub from: String,
    pub to: String,
    pub kind: NavigationKind,
}

#[derive(Default)]
struct WindowRouters(HashMap<WindowId, WeakEntity<Router>>);

impl Global for WindowRouters {}

pub struct Router {
    entries: Vec<SharedString>,
    index: usize,
    config: RouterConfig,
}

impl EventEmitter<NavigationEvent> for Router {}

impl Router {
    pub fn attach<T: 'static>(
        window: &mut Window,
        cx: &mut Context<T>,
        config: RouterConfig,
    ) -> Entity<Self> {
        Self::attach_at(window, cx, config, "/")
    }

    pub fn attach_at<T: 'static>(
        window: &mut Window,
        cx: &mut Context<T>,
        config: RouterConfig,
        path: impl Into<SharedString>,
    ) -> Entity<Self> {
        let location: SharedString = normalize_location(path.into().as_ref()).into();
        let router = cx.new(|_| Self {
            entries: vec![location],
            index: 0,
            config,
        });
        cx.default_global::<WindowRouters>()
            .0
            .insert(window.window_handle().window_id(), router.downgrade());
        router
    }

    pub fn location(&self) -> &str {
        &self.entries[self.index]
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    /// Push `path` onto the history stack.
    ///
    /// No-ops without emitting an event when `path` normalizes to the current
    /// location.
    pub fn navigate(&mut self, path: impl Into<SharedString>, cx: &mut Context<Self>) {
        let path = normalize_location(path.into().as_ref());
        if self.location() == path {
            return;
        }
        let from = self.location().to_owned();
        self.entries.truncate(self.index + 1);
        self.entries.push(path.clone().into());
        self.index = self.entries.len() - 1;
        cx.emit(NavigationEvent {
            from,
            to: path,
            kind: NavigationKind::Push,
        });
        cx.notify();
    }

    /// Replace the current history entry with `path`.
    ///
    /// No-ops without emitting an event when `path` normalizes to the current
    /// location.
    pub fn replace(&mut self, path: impl Into<SharedString>, cx: &mut Context<Self>) {
        let path = normalize_location(path.into().as_ref());
        if self.location() == path {
            return;
        }
        let from = self.location().to_owned();
        self.entries[self.index] = path.clone().into();
        cx.emit(NavigationEvent {
            from,
            to: path,
            kind: NavigationKind::Replace,
        });
        cx.notify();
    }

    /// Move back one history entry.
    ///
    /// No-ops without emitting an event when there is no previous entry.
    pub fn back(&mut self, cx: &mut Context<Self>) {
        if !self.can_go_back() {
            return;
        }
        let from = self.location().to_owned();
        self.index -= 1;
        cx.emit(NavigationEvent {
            from,
            to: self.location().to_owned(),
            kind: NavigationKind::Back,
        });
        cx.notify();
    }

    /// Move forward one history entry.
    ///
    /// No-ops without emitting an event when there is no next entry.
    pub fn forward(&mut self, cx: &mut Context<Self>) {
        if !self.can_go_forward() {
            return;
        }
        let from = self.location().to_owned();
        self.index += 1;
        cx.emit(NavigationEvent {
            from,
            to: self.location().to_owned(),
            kind: NavigationKind::Forward,
        });
        cx.notify();
    }

    /// Build a URL for a named route.
    ///
    /// Returns [`UrlError`] if the name is unknown or a required path parameter
    /// is missing. Extra query parameters are appended to the generated path.
    pub fn url<K, V>(
        &self,
        name: &str,
        params: impl IntoIterator<Item = (K, V)>,
    ) -> Result<String, UrlError>
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.config.url(name, params)
    }

    /// Navigate using the router attached to `window`.
    ///
    /// Returns `false` when this window has no router. Returns `true` after
    /// calling [`navigate`](Self::navigate), including when that call no-ops.
    pub fn navigate_window(window: &Window, cx: &mut App, path: impl Into<SharedString>) -> bool {
        update_window_router(window, cx, |router, cx| router.navigate(path, cx))
    }

    /// Replace using the router attached to `window`.
    ///
    /// Returns `false` when this window has no router. Returns `true` after
    /// calling [`replace`](Self::replace), including when that call no-ops.
    pub fn replace_window(window: &Window, cx: &mut App, path: impl Into<SharedString>) -> bool {
        update_window_router(window, cx, |router, cx| router.replace(path, cx))
    }

    /// Move back using the router attached to `window`.
    ///
    /// Returns `false` when this window has no router. Returns `true` after
    /// calling [`back`](Self::back), including when that call no-ops.
    pub fn back_window(window: &Window, cx: &mut App) -> bool {
        update_window_router(window, cx, |router, cx| router.back(cx))
    }

    /// Move forward using the router attached to `window`.
    ///
    /// Returns `false` when this window has no router. Returns `true` after
    /// calling [`forward`](Self::forward), including when that call no-ops.
    pub fn forward_window(window: &Window, cx: &mut App) -> bool {
        update_window_router(window, cx, |router, cx| router.forward(cx))
    }

    pub(crate) fn window_location(window: &Window, cx: &App) -> Option<SharedString> {
        window_router(window, cx).map(|router| router.read(cx).location().to_owned().into())
    }

    pub(crate) fn window_url(
        window: &Window,
        cx: &App,
        name: &str,
        params: &[(String, String)],
    ) -> Option<Result<String, UrlError>> {
        window_router(window, cx).map(|router| router.read(cx).url(name, params.iter().cloned()))
    }
}

fn window_router(window: &Window, cx: &App) -> Option<Entity<Router>> {
    cx.try_global::<WindowRouters>()
        .and_then(|routers| routers.0.get(&window.window_handle().window_id()))
        .and_then(WeakEntity::upgrade)
}

fn update_window_router(
    window: &Window,
    cx: &mut App,
    update: impl FnOnce(&mut Router, &mut Context<Router>),
) -> bool {
    let Some(router) = window_router(window, cx) else {
        return false;
    };
    router.update(cx, update);
    true
}

impl Render for Router {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut redirects_remaining = self.config.routes.len();

        loop {
            let Some(matched) = self.config.match_route(self.location()) else {
                return Empty.into_any_element();
            };
            let guard_result = self.config.routes[matched.index]
                .guards
                .iter()
                .map(|guard| guard(matched.context.clone(), window, cx))
                .find(|result| !matches!(result, GuardResult::Allow));
            match guard_result {
                Some(GuardResult::Deny) => return Empty.into_any_element(),
                Some(GuardResult::Redirect(redirect)) => {
                    let Ok(path) = self.config.resolve_redirect(&redirect) else {
                        return Empty.into_any_element();
                    };
                    if self.location() == path || redirects_remaining == 0 {
                        return Empty.into_any_element();
                    }
                    redirects_remaining -= 1;
                    self.replace(path, cx);
                }
                Some(GuardResult::Allow) | None => {
                    let route = &self.config.routes[matched.index];
                    let context = matched.context;
                    let mut current = (route.factory)(context.clone(), window, cx);
                    for layout in route.layouts.iter().rev() {
                        push_outlet(cx, current);
                        current = layout(context.clone(), window, cx);
                    }
                    return current.into_any_element();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{ParentElement, TestAppContext, div, point, px, size};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    struct Root {
        router: Entity<Router>,
    }

    impl Render for Root {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.router.clone())
        }
    }

    fn config() -> RouterConfig {
        RouterConfig::new()
            .route("/", || "home")
            .route("/about", || "about")
            .route("/{*rest}", || "not found")
    }

    #[test]
    fn matches_static_and_catch_all_routes() {
        let config = config();
        assert_eq!(config.match_index("/"), Some(0));
        assert_eq!(config.match_index("/about"), Some(1));
        assert_eq!(config.match_index("/missing/path"), Some(2));
    }

    #[test]
    fn returns_none_without_a_matching_route() {
        let config = RouterConfig::new().route("/", || "home");
        assert_eq!(config.match_index("/missing"), None);
    }

    #[gpui::test]
    async fn router_defaults_to_root_and_navigates(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let root = window.root(cx).unwrap();
        let router = root.read_with(cx, |root, _| root.router.clone());

        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/"
        );
        router.update(cx, |router, cx| router.navigate("/about", cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );

        router.update(cx, |router, cx| {
            router.navigate("//dashboard///settings/", cx)
        });
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/dashboard/settings"
        );
    }

    #[gpui::test]
    async fn routers_are_independent_per_window(cx: &mut TestAppContext) {
        let first = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let second = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let first_router = first
            .root(cx)
            .unwrap()
            .read_with(cx, |root, _| root.router.clone());
        let second_router = second
            .root(cx)
            .unwrap()
            .read_with(cx, |root, _| root.router.clone());

        first_router.update(cx, |router, cx| router.navigate("/about", cx));
        assert_eq!(
            first_router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );
        assert_eq!(
            second_router.read_with(cx, |router, _| router.location().to_owned()),
            "/"
        );
    }

    #[gpui::test]
    async fn window_navigation_resolves_the_attached_router(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        assert!(visual.update(|window, cx| { Router::navigate_window(window, cx, "/about") }));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/about"
        );
    }

    #[gpui::test]
    async fn attach_at_starts_on_the_given_path(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach_at(window, cx, config(), "/about"),
        });
        let router = router_for_window(&window, cx);

        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );
        assert!(!router.read_with(cx, |router, _| router.can_go_back()));
        assert!(!router.read_with(cx, |router, _| router.can_go_forward()));
    }

    #[gpui::test]
    async fn navigate_pushes_history_and_back_returns_to_the_previous_path(
        cx: &mut TestAppContext,
    ) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);

        router.update(cx, |router, cx| router.navigate("/about", cx));
        router.update(cx, |router, cx| router.navigate("/dashboard/settings", cx));

        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/dashboard/settings"
        );
        assert!(router.read_with(cx, |router, _| router.can_go_back()));
        assert!(!router.read_with(cx, |router, _| router.can_go_forward()));

        router.update(cx, |router, cx| router.back(cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );
        assert!(router.read_with(cx, |router, _| router.can_go_back()));
        assert!(router.read_with(cx, |router, _| router.can_go_forward()));

        router.update(cx, |router, cx| router.back(cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/"
        );
        assert!(!router.read_with(cx, |router, _| router.can_go_back()));
        assert!(router.read_with(cx, |router, _| router.can_go_forward()));
    }

    #[gpui::test]
    async fn forward_restores_the_path_after_back(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);

        router.update(cx, |router, cx| router.navigate("/about", cx));
        router.update(cx, |router, cx| router.back(cx));
        router.update(cx, |router, cx| router.forward(cx));

        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );
        assert!(router.read_with(cx, |router, _| router.can_go_back()));
        assert!(!router.read_with(cx, |router, _| router.can_go_forward()));
    }

    #[gpui::test]
    async fn back_and_forward_are_noops_at_the_ends_of_history(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);

        router.update(cx, |router, cx| router.back(cx));
        router.update(cx, |router, cx| router.forward(cx));

        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/"
        );
        assert!(!router.read_with(cx, |router, _| router.can_go_back()));
        assert!(!router.read_with(cx, |router, _| router.can_go_forward()));
    }

    #[gpui::test]
    async fn navigating_to_the_current_path_does_not_push_history(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);

        router.update(cx, |router, cx| router.navigate("/about", cx));
        router.update(cx, |router, cx| router.navigate("/about/", cx));

        assert!(router.read_with(cx, |router, _| router.can_go_back()));
        router.update(cx, |router, cx| router.back(cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/"
        );
        router.update(cx, |router, cx| router.forward(cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );
        assert!(!router.read_with(cx, |router, _| router.can_go_forward()));
    }

    #[gpui::test]
    async fn replace_updates_the_current_entry_without_pushing(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);

        router.update(cx, |router, cx| router.replace("/about", cx));

        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );
        assert!(!router.read_with(cx, |router, _| router.can_go_back()));
        assert!(!router.read_with(cx, |router, _| router.can_go_forward()));
    }

    #[gpui::test]
    async fn navigating_after_back_discards_the_forward_stack(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);

        router.update(cx, |router, cx| router.navigate("/about", cx));
        router.update(cx, |router, cx| router.navigate("/missing", cx));
        router.update(cx, |router, cx| router.back(cx));
        router.update(cx, |router, cx| router.navigate("/login", cx));

        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/login"
        );
        assert!(!router.read_with(cx, |router, _| router.can_go_forward()));

        router.update(cx, |router, cx| router.back(cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/about"
        );
    }

    #[gpui::test]
    async fn redirecting_guards_replace_the_attempted_location(cx: &mut TestAppContext) {
        let config = RouterConfig::new()
            .route("/", || "home")
            .route("/account", || "account")
            .guard(|| crate::GuardResult::Redirect("/login".into()))
            .route("/login", || "login");
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/account", cx));
        visual.update(|window, cx| {
            router.update(cx, |router, cx| {
                router.render(window, cx).into_any_element()
            });
        });

        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/login"
        );
        assert!(router.read_with(&visual, |router, _| router.can_go_back()));
        assert!(!router.read_with(&visual, |router, _| router.can_go_forward()));

        router.update(&mut visual, |router, cx| router.back(cx));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/"
        );
        router.update(&mut visual, |router, cx| router.forward(cx));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/login"
        );
    }

    #[gpui::test]
    async fn window_history_helpers_resolve_the_attached_router(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        assert!(visual.update(|window, cx| Router::navigate_window(window, cx, "/about")));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/about"
        );
        assert!(visual.update(|window, cx| Router::back_window(window, cx)));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/"
        );
        assert!(visual.update(|window, cx| Router::forward_window(window, cx)));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/about"
        );
        assert!(visual.update(|window, cx| Router::replace_window(window, cx, "/login")));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/login"
        );
        assert!(router.read_with(&visual, |router, _| router.can_go_back()));
        visual.update(|window, cx| Router::back_window(window, cx));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/"
        );
    }

    #[gpui::test]
    async fn only_the_matched_factory_is_rendered(cx: &mut TestAppContext) {
        let home = Arc::new(AtomicUsize::new(0));
        let about = Arc::new(AtomicUsize::new(0));
        let home_factory = home.clone();
        let about_factory = about.clone();
        let config = RouterConfig::new()
            .route("/", move || {
                home_factory.fetch_add(1, Ordering::SeqCst);
                "home"
            })
            .route("/about", move || {
                about_factory.fetch_add(1, Ordering::SeqCst);
                "about"
            });

        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert!(home.load(Ordering::SeqCst) > 0);
        assert_eq!(about.load(Ordering::SeqCst), 0);
    }

    #[gpui::test]
    async fn matched_parameters_are_passed_to_the_page_factory(cx: &mut TestAppContext) {
        let captured_id = Arc::new(Mutex::new(None));
        let page_id = captured_id.clone();
        let config = RouterConfig::new().route("/users/{id}", move |route: crate::RouteContext| {
            *page_id.lock().unwrap() = route.param("id").map(str::to_owned);
            "user"
        });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/users/42", cx));
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert_eq!(captured_id.lock().unwrap().as_deref(), Some("42"));
    }

    #[gpui::test]
    async fn query_values_are_passed_to_the_page_factory(cx: &mut TestAppContext) {
        let captured_query = Arc::new(Mutex::new(None));
        let page_query = captured_query.clone();
        let config = RouterConfig::new().route("/search", move |route: crate::RouteContext| {
            *page_query.lock().unwrap() = route.query("q").map(str::to_owned);
            "search"
        });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| {
            router.navigate("/search?q=rooter", cx)
        });
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router.clone()
        });

        assert_eq!(captured_query.lock().unwrap().as_deref(), Some("rooter"));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/search?q=rooter"
        );
    }

    #[gpui::test]
    async fn query_changes_push_history_entries(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);

        router.update(cx, |router, cx| router.navigate("/search?q=a", cx));
        router.update(cx, |router, cx| router.navigate("/search?q=b", cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/search?q=b"
        );

        router.update(cx, |router, cx| router.back(cx));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/search?q=a"
        );

        router.update(cx, |router, cx| router.navigate("/search?q=a", cx));
        assert!(router.read_with(cx, |router, _| router.can_go_forward()));
        assert_eq!(
            router.read_with(cx, |router, _| router.location().to_owned()),
            "/search?q=a"
        );
    }

    #[gpui::test]
    async fn page_factories_can_parse_typed_parameters(cx: &mut TestAppContext) {
        let captured_id = Arc::new(Mutex::new(None));
        let page_id = captured_id.clone();
        let config = RouterConfig::new().route("/users/{id}", move |route: crate::RouteContext| {
            *page_id.lock().unwrap() = Some(route.param_as::<u64>("id").unwrap());
            "user"
        });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/users/42", cx));
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert_eq!(*captured_id.lock().unwrap(), Some(42));
    }

    #[gpui::test]
    async fn catch_all_parameters_are_passed_to_the_page_factory(cx: &mut TestAppContext) {
        let captured_path = Arc::new(Mutex::new(None));
        let page_path = captured_path.clone();
        let config =
            RouterConfig::new().route("/files/{*path}", move |route: crate::RouteContext| {
                *page_path.lock().unwrap() = route.param("path").map(str::to_owned);
                "file"
            });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| {
            router.navigate("/files/documents/2026/report.pdf", cx)
        });
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert_eq!(
            captured_path.lock().unwrap().as_deref(),
            Some("documents/2026/report.pdf")
        );
    }

    #[gpui::test]
    async fn denied_routes_do_not_render_the_page_factory(cx: &mut TestAppContext) {
        let renders = Arc::new(AtomicUsize::new(0));
        let page_renders = renders.clone();
        let config = RouterConfig::new()
            .route("/", move || {
                page_renders.fetch_add(1, Ordering::SeqCst);
                "private"
            })
            .guard(|| crate::GuardResult::Deny);
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert_eq!(renders.load(Ordering::SeqCst), 0);
    }

    #[gpui::test]
    async fn route_aware_guards_receive_the_matched_context(cx: &mut TestAppContext) {
        let captured = Arc::new(Mutex::new(None));
        let guard_captured = captured.clone();
        let config = RouterConfig::new().route("/users/{id}", || "user").guard(
            move |route: crate::RouteContext| {
                *guard_captured.lock().unwrap() = Some((
                    route.path().to_owned(),
                    route.param("id").map(str::to_owned),
                    route.query("tab").map(str::to_owned),
                ));
                crate::GuardResult::Allow
            },
        );
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| {
            router.navigate("/users/42?tab=billing", cx)
        });
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert_eq!(
            captured.lock().unwrap().as_ref(),
            Some(&(
                "/users/42".to_owned(),
                Some("42".to_owned()),
                Some("billing".to_owned())
            ))
        );
    }

    #[gpui::test]
    async fn route_aware_guards_can_deny_based_on_parameters(cx: &mut TestAppContext) {
        let renders = Arc::new(AtomicUsize::new(0));
        let page_renders = renders.clone();
        let config = RouterConfig::new()
            .route("/users/{id}", move || {
                page_renders.fetch_add(1, Ordering::SeqCst);
                "user"
            })
            .guard(|route: crate::RouteContext| {
                if route.param("id") == Some("42") {
                    crate::GuardResult::Allow
                } else {
                    crate::GuardResult::Deny
                }
            });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/users/7", cx));
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router.clone()
        });
        assert_eq!(renders.load(Ordering::SeqCst), 0);

        router.update(&mut visual, |router, cx| router.navigate("/users/42", cx));
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });
        assert!(renders.load(Ordering::SeqCst) > 0);
    }

    #[gpui::test]
    async fn redirecting_guards_change_the_current_location(cx: &mut TestAppContext) {
        let login_renders = Arc::new(AtomicUsize::new(0));
        let login_page_renders = login_renders.clone();
        let config = RouterConfig::new()
            .route("/", || "private")
            .guard(|| crate::GuardResult::Redirect("/login".into()))
            .route("/login", move || {
                login_page_renders.fetch_add(1, Ordering::SeqCst);
                "login"
            });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        visual.update(|window, cx| {
            router.update(cx, |router, cx| {
                router.render(window, cx).into_any_element()
            });
        });

        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/login"
        );
        assert!(login_renders.load(Ordering::SeqCst) > 0);
    }

    #[gpui::test]
    async fn named_guard_redirects_are_resolved_before_navigation(cx: &mut TestAppContext) {
        let config = RouterConfig::new()
            .route("/account", || "account")
            .guard(|| {
                crate::GuardResult::Redirect(
                    crate::Redirect::named("login").param("from", "account"),
                )
            })
            .route("/login", || "login")
            .name("login");
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/account", cx));
        visual.update(|window, cx| {
            router.update(cx, |router, cx| {
                router.render(window, cx).into_any_element()
            });
        });

        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/login?from=account"
        );
        assert_eq!(
            router.read_with(&visual, |router, _| {
                router.url("login", [("from", "account")])
            }),
            Ok("/login?from=account".into())
        );
    }

    #[gpui::test]
    async fn invalid_named_redirects_do_not_panic(cx: &mut TestAppContext) {
        let config = RouterConfig::new()
            .route("/", || "home")
            .route("/secret", || "secret")
            .guard(|| crate::GuardResult::Redirect(crate::Redirect::named("missing")));
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/secret", cx));
        visual.update(|window, cx| {
            router.update(cx, |router, cx| {
                router.render(window, cx).into_any_element()
            });
        });

        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/secret"
        );
        assert_eq!(
            router.read_with(&visual, |router, _| {
                router.url("missing", Vec::<(&str, &str)>::new())
            }),
            Err(crate::UrlError::UnknownName("missing".into()))
        );
    }

    #[gpui::test]
    async fn grouped_index_routes_render_lazily(cx: &mut TestAppContext) {
        let dashboard = Arc::new(AtomicUsize::new(0));
        let settings = Arc::new(AtomicUsize::new(0));
        let dashboard_factory = dashboard.clone();
        let settings_factory = settings.clone();
        let config = RouterConfig::new().group("dashboard", |routes| {
            routes
                .index(move || {
                    dashboard_factory.fetch_add(1, Ordering::SeqCst);
                    "dashboard"
                })
                .route("settings", move || {
                    settings_factory.fetch_add(1, Ordering::SeqCst);
                    "settings"
                })
        });

        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/dashboard", cx));
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router.clone()
        });
        assert!(dashboard.load(Ordering::SeqCst) > 0);
        assert_eq!(settings.load(Ordering::SeqCst), 0);

        router.update(&mut visual, |router, cx| {
            router.navigate("/dashboard/settings/", cx)
        });
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router.clone()
        });
        assert!(settings.load(Ordering::SeqCst) > 0);
    }

    #[gpui::test]
    async fn layouts_wrap_the_matched_page_through_an_outlet(cx: &mut TestAppContext) {
        let order = Arc::new(Mutex::new(Vec::new()));
        let layout_order = order.clone();
        let page_order = order.clone();
        let captured_id = Arc::new(Mutex::new(None));
        let layout_id = captured_id.clone();
        let config = RouterConfig::new().group("/users", |routes| {
            routes
                .layout(move |route: crate::RouteContext| {
                    layout_order.lock().unwrap().push("layout");
                    *layout_id.lock().unwrap() = route.param("id").map(str::to_owned);
                    crate::Outlet::new()
                })
                .route("{id}", move || {
                    page_order.lock().unwrap().push("page");
                    "user"
                })
        });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/users/42", cx));
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert_eq!(&order.lock().unwrap()[..2], ["page", "layout"]);
        assert_eq!(captured_id.lock().unwrap().as_deref(), Some("42"));
    }

    #[gpui::test]
    async fn nested_layouts_render_from_the_inside_out(cx: &mut TestAppContext) {
        let order = Arc::new(Mutex::new(Vec::new()));
        let outer_order = order.clone();
        let inner_order = order.clone();
        let page_order = order.clone();
        let config = RouterConfig::new()
            .layout(move || {
                outer_order.lock().unwrap().push("shell");
                crate::Outlet::new()
            })
            .group("/dashboard", |routes| {
                routes
                    .layout(move || {
                        inner_order.lock().unwrap().push("dashboard");
                        crate::Outlet::new()
                    })
                    .index(move || {
                        page_order.lock().unwrap().push("page");
                        "dashboard"
                    })
            });
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/dashboard", cx));
        visual.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
            router
        });

        assert_eq!(&order.lock().unwrap()[..3], ["page", "dashboard", "shell"]);
    }

    #[gpui::test]
    async fn navigate_emits_a_push_event(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);
        let events = collect_events(&router, cx);

        router.update(cx, |router, cx| router.navigate("/about", cx));

        assert_eq!(
            *events.lock().unwrap(),
            [NavigationEvent {
                from: "/".into(),
                to: "/about".into(),
                kind: NavigationKind::Push,
            }]
        );
    }

    #[gpui::test]
    async fn replace_back_and_forward_emit_their_kinds(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);
        let events = collect_events(&router, cx);

        router.update(cx, |router, cx| router.navigate("/about", cx));
        router.update(cx, |router, cx| router.replace("/login", cx));
        router.update(cx, |router, cx| router.back(cx));
        router.update(cx, |router, cx| router.forward(cx));

        assert_eq!(
            *events.lock().unwrap(),
            [
                NavigationEvent {
                    from: "/".into(),
                    to: "/about".into(),
                    kind: NavigationKind::Push,
                },
                NavigationEvent {
                    from: "/about".into(),
                    to: "/login".into(),
                    kind: NavigationKind::Replace,
                },
                NavigationEvent {
                    from: "/login".into(),
                    to: "/".into(),
                    kind: NavigationKind::Back,
                },
                NavigationEvent {
                    from: "/".into(),
                    to: "/login".into(),
                    kind: NavigationKind::Forward,
                },
            ]
        );
    }

    #[gpui::test]
    async fn noop_history_changes_do_not_emit_events(cx: &mut TestAppContext) {
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = router_for_window(&window, cx);
        let events = collect_events(&router, cx);

        router.update(cx, |router, cx| router.navigate("/", cx));
        router.update(cx, |router, cx| router.back(cx));
        router.update(cx, |router, cx| router.forward(cx));

        assert!(events.lock().unwrap().is_empty());
    }

    #[gpui::test]
    async fn guard_redirects_emit_a_replace_event(cx: &mut TestAppContext) {
        let config = RouterConfig::new()
            .route("/", || "home")
            .route("/account", || "account")
            .guard(|| crate::GuardResult::Redirect("/login".into()))
            .route("/login", || "login");
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = router_for_window(&window, cx);
        let events = collect_events(&router, cx);
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| router.navigate("/account", cx));
        visual.update(|window, cx| {
            router.update(cx, |router, cx| {
                router.render(window, cx).into_any_element()
            });
        });

        assert_eq!(
            *events.lock().unwrap(),
            [
                NavigationEvent {
                    from: "/".into(),
                    to: "/account".into(),
                    kind: NavigationKind::Push,
                },
                NavigationEvent {
                    from: "/account".into(),
                    to: "/login".into(),
                    kind: NavigationKind::Replace,
                },
            ]
        );
    }

    fn collect_events(
        router: &Entity<Router>,
        cx: &mut TestAppContext,
    ) -> Arc<Mutex<Vec<NavigationEvent>>> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        router.update(cx, |_, cx| {
            let entity = cx.entity();
            cx.subscribe(&entity, move |_, _, event: &NavigationEvent, _| {
                captured.lock().unwrap().push(event.clone());
            })
            .detach();
        });
        events
    }

    fn router_for_window(
        window: &gpui::WindowHandle<Root>,
        cx: &mut TestAppContext,
    ) -> Entity<Router> {
        window
            .root(cx)
            .unwrap()
            .read_with(cx, |root, _| root.router.clone())
    }
}
