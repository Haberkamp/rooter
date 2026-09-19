use gpui::prelude::*;
use gpui::{
    App, Application, Context, Entity, InteractiveElement, StatefulInteractiveElement, Window,
    WindowOptions, div,
};
use rooter::{ActiveMatch, NavLink, Outlet, RouteContext, Router, RouterConfig};

fn routes(sidebar: Entity<Sidebar>) -> RouterConfig {
    RouterConfig::new()
        .layout(app_shell)
        .route("/", home)
        .group("/dashboard", |routes| {
            routes
                .layout(move || dashboard(sidebar.clone()))
                .index(dashboard_home)
                .route("settings", settings)
                .group("users", |routes| {
                    routes
                        .layout(users_chrome)
                        .index(users_index)
                        .route("{id}", user_page)
                })
        })
        .route("/{*rest}", not_found)
}

fn app_shell() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .gap_4()
                .child(
                    NavLink::to("/")
                        .when_active(None, |link| link.underline())
                        .child("Home"),
                )
                .child(
                    NavLink::to("/dashboard")
                        .when_active(ActiveMatch::Partial, |link| link.underline())
                        .child("Dashboard"),
                )
                .child(
                    NavLink::to("/dashboard/settings")
                        .when_active(None, |link| link.underline())
                        .child("Settings"),
                )
                .child(
                    NavLink::to("/dashboard/users")
                        .when_active(ActiveMatch::Partial, |link| link.underline())
                        .child("Users"),
                )
                .child(
                    NavLink::to("/dashboard/users/42")
                        .when_active(None, |link| link.underline())
                        .child("User 42"),
                ),
        )
        .child(Outlet::new())
}

fn dashboard(sidebar: Entity<Sidebar>) -> impl IntoElement {
    div()
        .flex()
        .gap_4()
        .child(sidebar)
        .child(div().flex().flex_col().gap_2().child(Outlet::new()))
}

fn users_chrome(route: RouteContext) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(format!("Users chrome for {}.", route.path()))
        .child(Outlet::new())
}

fn home() -> impl IntoElement {
    div().child(
        "Home sits in the app shell only. Open Dashboard, collapse the sidebar or add a note, then leave and come back.",
    )
}

fn dashboard_home() -> impl IntoElement {
    div().child("This is the dashboard index. It is wrapped by the app shell and dashboard layout.")
}

fn settings() -> impl IntoElement {
    div().child("Settings uses the dashboard layout, not the users chrome.")
}

fn users_index() -> impl IntoElement {
    div().child("The users index is wrapped by both the dashboard layout and the users chrome.")
}

fn user_page(route: RouteContext) -> impl IntoElement {
    let id = route.param_as::<u64>("id").unwrap();
    div().child(format!("User {id} is rendered inside both nested layouts."))
}

fn not_found() -> impl IntoElement {
    div().child("This catch-all still uses the app shell.")
}

struct Sidebar {
    collapsed: bool,
    notes: usize,
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.collapsed;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(if collapsed {
                "Sidebar (collapsed)"
            } else {
                "Dashboard sidebar"
            })
            .child(format!("Remembered notes: {}", self.notes))
            .child(
                div()
                    .id("toggle-sidebar")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.collapsed = !this.collapsed;
                        cx.notify();
                    }))
                    .child(if collapsed { "Expand" } else { "Collapse" }),
            )
            .child(
                div()
                    .id("add-note")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.notes += 1;
                        cx.notify();
                    }))
                    .child("Add a note"),
            )
            .when(!collapsed, |this| {
                this.child("This layout wraps every /dashboard route. Its state lives on AppView, not in the layout factory.")
            })
    }
}

struct AppView {
    router: Entity<Router>,
}

impl AppView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|_| Sidebar {
            collapsed: false,
            notes: 0,
        });
        Self {
            router: Router::attach(window, cx, routes(sidebar)),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .bg(gpui::white())
            .text_color(gpui::black())
            .child(self.router.clone())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.activate(true);
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| AppView::new(window, cx))
        })
        .unwrap();
    });
}
