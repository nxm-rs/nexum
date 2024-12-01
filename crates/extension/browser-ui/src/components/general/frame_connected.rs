use leptos::*;
use stylers::style;

use crate::helper::StringOrMap;

#[component]
pub fn FrameConnected(
    #[prop(optional)] style: Option<StringOrMap>,
    children: Children,
) -> impl IntoView {
    // Define the main styles with `stylers`
    let style = style.map(|style| style.to_string()).unwrap_or_default();
    let styler_class = style! { "FrameConnected",
        .frame-connected {
            font-size: 14px;
            text-transform: uppercase;
            font-weight: 600;
            letter-spacing: 1px;
            padding-left: 1px;
        }
    };

    view! { class=styler_class,
        <div class="frame-connected" style=style>
            {children()}
        </div>
    }
}
