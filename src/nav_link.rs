use crate::{Router, config::location_path};
use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Stateful, StatefulInteractiveElement, Window, div,
};

type ActiveStyle = Box<dyn FnOnce(Stateful<Div>) -> Stateful<Div>>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActiveMatch {
    #[default]
    Exact,
    Partial,
}

#[derive(Clone, Debug)]
enum NavTarget {
    Path(SharedString),
    Named {
        name: SharedString,
        params: Vec<(String, String)>,
    },
}

#[derive(IntoElement)]
pub struct NavLink {
    to: NavTarget,
    children: Vec<AnyElement>,
    match_mode: ActiveMatch,
    when_active: Option<ActiveStyle>,
}

impl NavLink {
    pub fn to(to: impl Into<SharedString>) -> Self {
        Self {
            to: NavTarget::Path(to.into()),
            children: Vec::new(),
            match_mode: ActiveMatch::Exact,
            when_active: None,
        }
    }

    pub fn named(name: impl Into<SharedString>) -> Self {
        Self {
            to: NavTarget::Named {
                name: name.into(),
                params: Vec::new(),
            },
            children: Vec::new(),
            match_mode: ActiveMatch::Exact,
            when_active: None,
        }
    }

    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let NavTarget::Named { params, .. } = &mut self.to {
            params.push((key.into(), value.into()));
        } else {
            panic!("NavLink::param can only be used with NavLink::named");
        }
        self
    }

    pub fn when_active(
        mut self,
        mode: impl Into<Option<ActiveMatch>>,
        style: impl FnOnce(Stateful<Div>) -> Stateful<Div> + 'static,
    ) -> Self {
        self.match_mode = mode.into().unwrap_or_default();
        self.when_active = Some(Box::new(style));
        self
    }
}

impl ParentElement for NavLink {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for NavLink {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let to = resolve_target(&self.to, window, cx);
        let active = Router::window_location(window, cx).is_some_and(|location| {
            location_matches(location.as_ref(), to.as_ref(), self.match_mode)
        });
        let mut link = div()
            .id(ElementId::from(to.clone()))
            .on_click(move |_, window, cx| {
                Router::navigate_window(window, cx, to.clone());
            })
            .children(self.children);
        if active && let Some(when_active) = self.when_active {
            link = when_active(link);
        }
        link
    }
}

fn resolve_target(target: &NavTarget, window: &Window, cx: &App) -> SharedString {
    match target {
        NavTarget::Path(path) => path.clone(),
        NavTarget::Named { name, params } => Router::window_url(window, cx, name, params)
            .unwrap_or_else(|| panic!("unknown route name `{name}`"))
            .into(),
    }
}

fn location_matches(location: &str, to: &str, mode: ActiveMatch) -> bool {
    let location = location_path(location);
    let to = location_path(to);
    match mode {
        ActiveMatch::Partial => {
            to == "/" || location == to || location.starts_with(&format!("{to}/"))
        }
        ActiveMatch::Exact => location == to,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Router, RouterConfig};
    use gpui::{Context, Entity, ParentElement, Render, TestAppContext, point, px, size};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
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
            .route("/dashboard", || "dashboard")
            .route("/dashboard/users", || "users")
    }

    #[test]
    fn exact_matches_normalize_the_location_and_target() {
        assert!(location_matches("/about/", "/about", ActiveMatch::Exact));
        assert!(!location_matches(
            "/about",
            "/dashboard",
            ActiveMatch::Exact
        ));
        assert!(!location_matches(
            "/dashboard/users",
            "/dashboard",
            ActiveMatch::Exact
        ));
        assert!(!location_matches("/about", "/", ActiveMatch::Exact));
        assert!(location_matches(
            "/search?q=rooter",
            "/search",
            ActiveMatch::Exact
        ));
        assert!(!location_matches(
            "/search?q=rooter",
            "/about",
            ActiveMatch::Exact
        ));
    }

    #[test]
    fn partial_matches_nested_paths_but_not_sibling_paths() {
        assert!(location_matches(
            "/dashboard",
            "/dashboard",
            ActiveMatch::Partial
        ));
        assert!(location_matches(
            "/dashboard/users",
            "/dashboard",
            ActiveMatch::Partial
        ));
        assert!(location_matches(
            "/dashboard/users/42",
            "/dashboard/users",
            ActiveMatch::Partial
        ));
        assert!(!location_matches(
            "/dashboard-admin",
            "/dashboard",
            ActiveMatch::Partial
        ));
        assert!(location_matches("/about", "/", ActiveMatch::Partial));
    }

    #[gpui::test]
    async fn applies_active_styles_only_for_the_matching_link(cx: &mut TestAppContext) {
        let home_active = Arc::new(AtomicBool::new(false));
        let about_active = Arc::new(AtomicBool::new(false));
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = window
            .root(cx)
            .unwrap()
            .read_with(cx, |root, _| root.router.clone());
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        let draw = |visual: &mut gpui::VisualTestContext,
                    router: Entity<Router>,
                    home_active: Arc<AtomicBool>,
                    about_active: Arc<AtomicBool>| {
            home_active.store(false, Ordering::SeqCst);
            about_active.store(false, Ordering::SeqCst);
            visual.draw(point(px(0.), px(0.)), size(px(200.), px(40.)), |_, _| {
                let home = home_active.clone();
                let about = about_active.clone();
                div()
                    .child(router)
                    .child(NavLink::to("/").when_active(None, move |link| {
                        home.store(true, Ordering::SeqCst);
                        link
                    }))
                    .child(NavLink::to("/about").when_active(None, move |link| {
                        about.store(true, Ordering::SeqCst);
                        link
                    }))
            });
        };

        draw(
            &mut visual,
            router.clone(),
            home_active.clone(),
            about_active.clone(),
        );
        assert!(home_active.load(Ordering::SeqCst));
        assert!(!about_active.load(Ordering::SeqCst));

        router.update(&mut visual, |router, cx| router.navigate("/about", cx));
        draw(
            &mut visual,
            router,
            home_active.clone(),
            about_active.clone(),
        );
        assert!(!home_active.load(Ordering::SeqCst));
        assert!(about_active.load(Ordering::SeqCst));
    }

    #[gpui::test]
    async fn partial_links_stay_active_on_nested_routes(cx: &mut TestAppContext) {
        let dashboard_active = Arc::new(AtomicBool::new(false));
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config()),
        });
        let router = window
            .root(cx)
            .unwrap()
            .read_with(cx, |root, _| root.router.clone());
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| {
            router.navigate("/dashboard/users", cx)
        });
        visual.draw(point(px(0.), px(0.)), size(px(200.), px(40.)), |_, _| {
            let dashboard = dashboard_active.clone();
            div().child(
                NavLink::to("/dashboard").when_active(ActiveMatch::Partial, move |link| {
                    dashboard.store(true, Ordering::SeqCst);
                    link
                }),
            )
        });

        assert!(dashboard_active.load(Ordering::SeqCst));
    }

    #[gpui::test]
    async fn named_links_resolve_to_generated_paths(cx: &mut TestAppContext) {
        let user_active = Arc::new(AtomicBool::new(false));
        let config = RouterConfig::new()
            .route("/", || "home")
            .route("/users/{id}", || "user")
            .name("users.show");
        let window = cx.add_window(|window, cx| Root {
            router: Router::attach(window, cx, config),
        });
        let router = window
            .root(cx)
            .unwrap()
            .read_with(cx, |root, _| root.router.clone());
        let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);

        router.update(&mut visual, |router, cx| {
            let path = router.url("users.show", [("id", "42"), ("tab", "profile")]);
            router.navigate(path, cx);
        });
        visual.draw(point(px(0.), px(0.)), size(px(200.), px(40.)), |_, _| {
            let user = user_active.clone();
            div().child(
                NavLink::named("users.show")
                    .param("id", "42")
                    .param("tab", "profile")
                    .when_active(None, move |link| {
                        user.store(true, Ordering::SeqCst);
                        link
                    }),
            )
        });

        assert!(user_active.load(Ordering::SeqCst));
        assert_eq!(
            router.read_with(&visual, |router, _| router.location().to_owned()),
            "/users/42?tab=profile"
        );
    }
}
