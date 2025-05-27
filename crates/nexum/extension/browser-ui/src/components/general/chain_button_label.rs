use leptos::*;
use stylers::style;

#[component]
pub fn ChainButtonLabel(children: Children) -> impl IntoView {
    let styler_class = style! {
        "ChainButtonLabel",
        .chain-button-label {
            display: flex;
            justify-content: center;
            align-items: center;
            flex-grow: 1;
            font-size: 14px;
            padding-left: 4px;
            font-weight: 500;
            height: 44px;
        }
    };

    view! { class=styler_class, <div class="chain-button-label">{children()}</div> }
}
