use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Window, WindowOptions, div};
use rooter::{NavLink, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", |_, _| page("This page is outside every group."))
        .group("/dashboard", |routes| {
            routes
                .index(|_, _| page("This index page matches the /dashboard group itself."))
                .route("settings", |_, _| {
                    page("This route expands to /dashboard/settings.")
                })
                .group("users", |routes| {
                    routes
                        .index(|_, _| page("This nested index matches /dashboard/users."))
                        .route("{id}", |_, _| {
                            page("This dynamic route matches a single user id.")
                        })
                        .route("{*rest}", |_, _| {
                            page("This catch-all is scoped to /dashboard/users.")
                        })
                })
        })
        .route("/{*rest}", |_, _| {
            page("This is the application catch-all.")
        })
}

fn page(body: &'static str) -> impl IntoElement {
    div().child(body)
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
