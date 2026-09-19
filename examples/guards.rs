use gpui::prelude::*;
use gpui::{
    App, AppContext, Application, BorrowAppContext, Context, Entity, Global, Window, WindowOptions,
    div,
};
use rooter::{GuardResult, NavLink, Redirect, RouteContext, Router, RouterConfig};

#[derive(Default)]
struct AuthState {
    logged_in: bool,
}

impl Global for AuthState {}

fn authenticated(_window: &mut Window, cx: &mut App) -> GuardResult {
    if cx.global::<AuthState>().logged_in {
        GuardResult::Allow
    } else {
        GuardResult::Redirect(Redirect::named("login"))
    }
}

fn owns_account(route: RouteContext) -> GuardResult {
    if route.param("id") == Some("42") {
        GuardResult::Allow
    } else {
        GuardResult::Redirect(Redirect::named("account"))
    }
}

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", || {
            div().child("This page is public. Try opening the protected account page.")
        })
        .route("/login", || {
            div().child("You must log in before viewing the protected page.")
        })
        .name("login")
        .route("/account", || {
            div().child("You are logged in, so the account guard allowed this page.")
        })
        .name("account")
        .guard(authenticated)
        .route("/users/{id}", || {
            div().child("This page is only allowed for user 42.")
        })
        .name("users.show")
        .guard(authenticated)
        .guard(owns_account)
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
        let logged_in = cx.global::<AuthState>().logged_in;
        let router = self.router.clone();

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
                        NavLink::named("account")
                            .when_active(None, |link| link.underline())
                            .child("Protected account"),
                    )
                    .child(
                        NavLink::named("users.show")
                            .param("id", "42")
                            .when_active(None, |link| link.underline())
                            .child("User 42"),
                    )
                    .child(
                        NavLink::named("users.show")
                            .param("id", "7")
                            .when_active(None, |link| link.underline())
                            .child("User 7"),
                    ),
            )
            .child(
                div()
                    .id("auth-toggle")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.update_global::<AuthState, _>(|auth, _| {
                            auth.logged_in = !auth.logged_in;
                        });
                        let destination = if logged_in { "/" } else { "/account" };
                        router.update(cx, |router, cx| router.navigate(destination, cx));
                        cx.notify();
                    }))
                    .child(if logged_in { "Log out" } else { "Log in" }),
            )
            .child(if logged_in {
                "Status: logged in"
            } else {
                "Status: logged out"
            })
            .child(self.router.clone())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.set_global(AuthState::default());
        cx.activate(true);
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| AppView::new(window, cx))
        })
        .unwrap();
    });
}
