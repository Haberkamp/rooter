use gpui::{AnyElement, App, Empty, Global, IntoElement, RenderOnce, Window};

#[derive(Default)]
struct OutletStack(Vec<AnyElement>);

impl Global for OutletStack {}

pub(crate) fn push_outlet(cx: &mut App, element: AnyElement) {
    cx.default_global::<OutletStack>().0.push(element);
}

fn take_outlet(cx: &mut App) -> AnyElement {
    cx.default_global::<OutletStack>()
        .0
        .pop()
        .unwrap_or_else(|| Empty.into_any_element())
}

#[derive(IntoElement)]
pub struct Outlet;

impl Outlet {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Outlet {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Outlet {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        take_outlet(cx)
    }
}
