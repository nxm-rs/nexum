use leptos::*;
use stylers::style;

use crate::helper::StringOrMap;

#[component]
pub fn Row(#[prop(optional)] style: Option<StringOrMap>, children: Children) -> impl IntoView {
    // Convert custom_style to a CSS string if it's provided as a HashMap
    let style = style.map(|style| style.to_string()).unwrap_or_default();
    let styler_class = style! {"Row",
        .cluster-row {
            display: flex;
            justify-content: center;
            align-items: stretch;
            font-weight: 300;
            margin-left: 3px;
        }
    };

    view! { class=styler_class,
        <div class="cluster-row" style=style>
            {children()}
        </div>
    }
}
