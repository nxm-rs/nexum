use leptos::*;
use stylers::style;

#[component]
pub fn Tag(children: Children) -> impl IntoView {
    let styler_class = style! {"Tag",
        .cluster-tag {
            text-transform: uppercase;
            font-size: 11px;
            font-weight: 500;
            padding: 8px;
            text-align: center;
        }
    };

    view! { class=styler_class, <div class="cluster-tag">{children()}</div> }
}
