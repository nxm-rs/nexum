use leptos::prelude::*;
use stylers::style;

mod address;
pub use address::*;
mod address_recipient;
pub use address_recipient::*;
mod address_recipient_full;
pub use address_recipient_full::*;
mod box_label;
pub use box_label::*;
mod box_main;
pub use box_main::*;
mod column;
pub use column::*;
mod fira;
pub use fira::*;
mod focus;
pub use focus::*;
mod focus_highlight;
pub use focus_highlight::*;
mod row;
pub use row::*;
mod tag;
pub use tag::*;
mod value;
pub use value::*;

#[component]
pub fn Cluster(children: Children) -> impl IntoView {
    let styler_class = style! {"Cluster",
        .cluster {
            font-size: 17px;
            font-weight: 400;
            border-radius: 20px;
            // -webkit-app-region: no-drag;
            transform: translate3d(0, 0, 0);
            font-family: "MainFont";
            display: flow-root;
            background: var(--ghostZ);
            margin: 6px;
            padding: 2px 0px 1px 0px;
        }
    };

    view! { class=styler_class, <div class="cluster">{children()}</div> }
}
