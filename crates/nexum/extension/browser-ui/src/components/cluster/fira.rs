use leptos::prelude::*;
use stylers::style;

#[component]
pub fn Fira(children: Children) -> impl IntoView {
    let styler_class = style! {"Fira",
        .cluster-fira {
            position: relative;
            top: 1.5px;
            left: 1px;
            font-weight: 300;
            font-size: 13px;
            font-family: "FiraCode";
        }
    };

    view! { class=styler_class, <div class="cluster-fira">{children()}</div> }
}
