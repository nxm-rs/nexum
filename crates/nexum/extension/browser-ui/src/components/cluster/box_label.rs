use leptos::*;
use stylers::style;

#[component]
pub fn BoxLabel(children: Children) -> impl IntoView {
    let styler_class = style! {"BoxLabel",
        .cluster-box-label {
            position: relative;
            font-size: 16px;
            padding: 16px 0px 8px 0px;
            font-family: "MainFont";
            font-weight: 400;
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--moon);
        }
    };

    view! { class=styler_class, <div class="cluster-box-label">{children()}</div> }
}
