use leptos::*;
use stylers::style;

#[component]
pub fn Column(children: Children) -> impl IntoView {
    let styler_class = style! {"Column",
        .cluster-column {
            display: flex;
            flex-direction: column;
            flex-grow: 1;
            font-size: 14px;
            align-items: stretch;
        }
    };

    view! { class=styler_class, <div class="cluster-column">{children()}</div> }
}
