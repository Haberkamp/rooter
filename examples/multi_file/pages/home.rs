use gpui::div;
use gpui::prelude::*;

pub fn page() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("Welcome to the multi-file rooter example.")
        .child("Every page in this example lives in its own Rust file.")
}
