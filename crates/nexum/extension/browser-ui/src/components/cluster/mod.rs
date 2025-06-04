use leptos::prelude::*;
use stylers::style;

#[allow(dead_code)]
mod address;
#[allow(dead_code)]
mod address_recipient;
#[allow(dead_code)]
mod address_recipient_full;
#[allow(dead_code)]
mod box_label;
mod box_main;
pub use box_main::*;
#[allow(dead_code)]
mod column;
#[allow(dead_code)]
mod fira;
#[allow(dead_code)]
mod focus;
#[allow(dead_code)]
mod focus_highlight;
mod row;
pub use row::*;
#[allow(dead_code)]
mod tag;
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
