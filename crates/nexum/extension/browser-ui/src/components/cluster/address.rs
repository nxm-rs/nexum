use leptos::*;
use stylers::style;

#[component]
pub fn Address(children: Children) -> impl IntoView {
    let styler_class = style! {"Address",
        .cluster-address {
            padding: 12px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
        }
    };

    view! { class=styler_class, <div class="cluster-address">{children()}</div> }
}
