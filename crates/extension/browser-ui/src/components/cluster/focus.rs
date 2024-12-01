use leptos::*;
use stylers::style;

#[component]
pub fn Focus(children: Children) -> impl IntoView {
    let styler_class = style! {"Focus",
        .cluster-focus {
            text-transform: uppercase;
            font-size: 13px;
            line-height: 20px;
            font-weight: 500;
            padding: 16px 8px;
            text-align: center;
        }
    };

    view! { class=styler_class, <div class="cluster-focus">{children()}</div> }
}
