use leptos::*;
use stylers::style;

use crate::helper::StringOrMap;

#[component]
pub fn BoxMain(#[prop(optional)] style: Option<StringOrMap>, children: Children) -> impl IntoView {
    let style = style.map(|style| style.to_string()).unwrap_or_default();
    let styler_class = style! {"BoxMain",
        .cluster-box-main {
            position: relative;
            z-index: 100001;
            border-radius: 26px;
            overflow: hidden;
            box-shadow:
                0px 4px 8px var(--ghostY),
                0px 2px 8px var(--ghostY);
            border-bottom: 2px solid var(--ghostZ);
            padding: 0;
            text-align: center;
            margin: 6px;
            box-sizing: border-box;
            background: var(--ghostAZ);
            user-select: none;
        }
    };

    view! { class=styler_class,
        <div class="cluster-box-main" style=style>
            {children()}
        </div>
    }
}
