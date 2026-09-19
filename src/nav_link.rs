use crate::Router;
use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Window, div,
};

#[derive(IntoElement)]
pub struct NavLink {
    to: SharedString,
    children: Vec<AnyElement>,
}

impl NavLink {
    pub fn to(to: impl Into<SharedString>) -> Self {
        Self {
            to: to.into(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for NavLink {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for NavLink {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let to = self.to;
        div()
            .id(ElementId::from(to.clone()))
            .on_click(move |_, window, cx| {
                Router::navigate_window(window, cx, to.clone());
            })
            .children(self.children)
    }
}
