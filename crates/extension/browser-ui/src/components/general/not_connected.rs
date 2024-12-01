use leptos::*;
use stylers::style;

#[component]
pub fn NotConnected(children: Children) -> impl IntoView {
    let styler_class = style! {"NotConnected",
        .not-connected {
            padding: 32px;
            display: flex;
            justify-content: center;
            align-items: center;
            font-size: 18px;
        }
    };

    view! { class=styler_class, <div class="not-connected">{children()}</div> }
}
