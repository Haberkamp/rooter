use std::time::Duration;

use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Timer, Window, WindowOptions, div};
use rooter::{NavLink, PrefetchWhen, Resource, RouteContext, Router, RouterConfig};

struct User {
    name: String,
}

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", home)
        .route("/users/{id}", user_page)
        .loader(load_user)
        .route("/{*rest}", not_found)
}

fn home() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("Each user page waits 2 seconds before the loader resolves.")
        .child("Hover User 42 for about a second, then click. The wait should already be underway.")
        .child("Click User 7 without hovering. That page should sit on Loading for the full 2 seconds.")
        .child("Hover prefetch caches for 5 seconds. A visit without prefetch uses the 30s default. After expiry, the same user loads again.")
}

async fn load_user(route: RouteContext) -> Result<User, String> {
    let id = route.param("id").unwrap().to_owned();
    Timer::after(Duration::from_secs(2)).await;
    if id == "missing" {
        Err(format!("user {id} was not found"))
    } else {
        Ok(User {
            name: format!("User {id}"),
        })
    }
}

fn user_page(user: Resource<User, String>, route: RouteContext) -> impl IntoElement {
    let id = route.param("id").unwrap().to_owned();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(format!("User {id}"))
        .child(if user.is_loading() {
            format!("Loading user {id}… (artificial 2s delay)")
        } else if let Some(error) = user.error() {
            error.clone()
        } else {
            format!("Hello, {}.", user.get().unwrap().name)
        })
}

fn not_found() -> impl IntoElement {
    div().child("This example only has Home and the user pages.")
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let location = self.router.read(cx).location().to_owned();

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
                    .child(
                        NavLink::to("/")
                            .when_active(None, |link| link.underline())
                            .child("Home"),
                    )
                    .child(
                        NavLink::to("/users/42")
                            .prefetch(PrefetchWhen::Hover, Duration::from_secs(5))
                            .when_active(None, |link| link.underline())
                            .child("User 42"),
                    )
                    .child(
                        NavLink::to("/users/7")
                            .prefetch(PrefetchWhen::Hover, Duration::from_secs(5))
                            .when_active(None, |link| link.underline())
                            .child("User 7"),
                    )
                    .child(
                        NavLink::to("/users/missing")
                            .prefetch(PrefetchWhen::Hover, Duration::from_secs(5))
                            .when_active(None, |link| link.underline())
                            .child("Missing user"),
                    ),
            )
            .child(format!("Location: {location}"))
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
