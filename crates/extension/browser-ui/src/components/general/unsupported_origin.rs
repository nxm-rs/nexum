use leptos::*;
use stylers::style;

#[component]
pub fn UnsupportedOrigin(children: Children) -> impl IntoView {
    let styler_class = style! {"UnsupportedOrigin",
        .unsupported-origin {
            color: var(--moon);
            padding-top: 4px;
            padding-bottom: 4px;
            font-size: 18px;
        }
    };

    view! { class=styler_class, <div class="unsupported-origin">{children()}</div> }
}
