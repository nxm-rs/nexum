use leptos::prelude::*;
use stylers::style;

#[component]
pub fn FocusHightlight(children: Children) -> impl IntoView {
    let styler_class = style! {"FocusHightlight",
        .cluster-focus-highlight {
            font-size: 16px;
            color: var(--good);
        }
    };

    view! { class=styler_class, <div class="cluster-focus-highlight">{children()}</div> }
}
