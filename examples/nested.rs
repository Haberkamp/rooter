use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Window, WindowOptions, div};
use rooter::{NavLink, RouteContext, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", || page("This page is outside every group."))
        .group("/dashboard", |routes| {
            routes
                .index(|| page("This index page matches the /dashboard group itself."))
                .route("settings", || {
                    page("This route expands to /dashboard/settings.")
                })
                .group("users", |routes| {
                    routes
                        .index(|| page("This nested index matches /dashboard/users."))
                        .route("{id}", user_page)
                        .route("{*rest}", missing_user_page)
                })
        })
        .route("/{*rest}", || page("This is the application catch-all."))
}

fn page(body: impl IntoElement) -> impl IntoElement {
    div().child(body)
}

fn user_page(route: RouteContext) -> impl IntoElement {
    let id = route.param_as::<u64>("id").unwrap();
    page(format!("This dynamic route matched numeric user id {id}."))
}

fn missing_user_page(route: RouteContext) -> impl IntoElement {
    page(format!(
        "No user route matched the remaining path: {}.",
        route.param("rest").unwrap()
    ))
}

struct AppView {
    router: Entity<Router>,
}

impl AppView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            router: Router::attach(window, cx, routes()),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(gpui::white())
            .text_color(gpui::black())
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(NavLink::to("/").child("Home"))
                    .child(NavLink::to("/dashboard").child("Dashboard"))
                    .child(NavLink::to("/dashboard/settings").child("Settings"))
                    .child(NavLink::to("/dashboard/users").child("Users"))
                    .child(NavLink::to("/dashboard/users/42").child("User 42")),
            )
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
