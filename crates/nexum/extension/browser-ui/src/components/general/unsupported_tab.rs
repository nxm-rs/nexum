use leptos::*;
use stylers::style;

#[component]
pub fn UnsupportedTab(children: Children) -> impl IntoView {
    let styler_class = style! {"UnsupportedTab",
        .unsupported-tab {
            color: var(--moon);
            padding-top: 4px;
            padding-bottom: 4px;
            font-size: 18px;
        }
    };

    view! { class=styler_class, <div class="unsupported-tab">{children()}</div> }
}
