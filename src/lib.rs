mod config;
mod nav_link;
mod outlet;
mod resource;
mod router;

pub use config::{
    GuardFactory, GuardResult, PageFactory, ParamConstraint, ParamError, Redirect, RouteContext,
    RouterConfig, UrlError,
};
#[doc(hidden)]
pub use config::{
    WithAppContext, WithResource, WithResourceRoute, WithResourceRouteContext, WithRoute,
    WithRouteContext, WithoutContext,
};
pub use nav_link::{ActiveMatch, NavLink, PrefetchWhen};
#[doc(hidden)]
pub use nav_link::{NamedTarget, PathTarget};
pub use outlet::Outlet;
pub use resource::{DEFAULT_CACHE_FOR, IntoCacheFor, Invalidate, LoaderFactory, Resource};
pub use router::{NavigationEvent, NavigationKind, Router};
