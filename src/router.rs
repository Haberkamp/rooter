use crate::{GuardResult, RouterConfig, config::normalize_path};
use gpui::{
    App, AppContext, Context, Empty, Entity, Global, IntoElement, Render, SharedString, WeakEntity,
    Window, WindowId,
};
use std::collections::HashMap;

#[derive(Default)]
struct WindowRouters(HashMap<WindowId, WeakEntity<Router>>);

impl Global for WindowRouters {}

pub struct Router {
    location: SharedString,
    #[allow(dead_code)]
    config: RouterConfig,
}

impl Router {
    pub fn attach<T: 'static>(
        window: &mut Window,
        cx: &mut Context<T>,
        config: RouterConfig,
    ) -> Entity<Self> {
        let router = cx.new(|_| Self {
            location: "/".into(),
            config,
        });
        cx.default_global::<WindowRouters>()
            .0
            .insert(window.window_handle().window_id(), router.downgrade());
        router
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn navigate(&mut self, path: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.location = normalize_path(path.into().as_ref()).into();
        cx.notify();
    }

    pub fn navigate_window(window: &Window, cx: &mut App, path: impl Into<SharedString>) -> bool {
        let router = cx
            .try_global::<WindowRouters>()
            .and_then(|routers| routers.0.get(&window.window_handle().window_id()))
            .and_then(WeakEntity::upgrade);
        let Some(router) = router else {
            return false;
        };
        let path = path.into();
        router.update(cx, |router, cx| router.navigate(path, cx));
        true
    }

    pub(crate) fn window_location(window: &Window, cx: &App) -> Option<SharedString> {
        cx.try_global::<WindowRouters>()
            .and_then(|routers| routers.0.get(&window.window_handle().window_id()))
            .and_then(WeakEntity::upgrade)
            .map(|router| router.read(cx).location.clone())
    }
}

impl Render for Router {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut redirects_remaining = self.config.routes.len();

        loop {
            let Some(matched) = self.config.match_route(&self.location) else {
                return Empty.into_any_element();
            };
            let guard_result = self.config.routes[matched.index]
                .guards
                .iter()
                .map(|guard| guard(window, cx))
                .find(|result| !matches!(result, GuardResult::Allow));
            match guard_result {
                Some(GuardResult::Deny) => return Empty.into_any_element(),
                Some(GuardResult::Redirect(path)) => {
                    let path = normalize_path(&path);
                    if self.location.as_ref() == path || redirects_remaining == 0 {
                        return Empty.into_any_element();
                    }
                    redirects_remaining -= 1;
                    self.location = path.into();
                    cx.notify();
                }
                Some(GuardResult::Allow) | None => {
                    return (self.config.routes[matched.index].factory)(
                        matched.context,
                        window,
                        cx,
                    )
                    .into_any_element();
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
            .guard(|_, _| crate::GuardResult::Deny);
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
    async fn redirecting_guards_change_the_current_location(cx: &mut TestAppContext) {
        let login_renders = Arc::new(AtomicUsize::new(0));
        let login_page_renders = login_renders.clone();
        let config = RouterConfig::new()
            .route("/", || "private")
            .guard(|_, _| crate::GuardResult::Redirect("/login".to_owned()))
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
