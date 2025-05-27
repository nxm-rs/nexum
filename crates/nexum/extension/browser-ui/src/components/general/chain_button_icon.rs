use leptos::prelude::*;
use stylers::style;

#[component]
pub fn ChainButtonIcon(#[prop(optional)] selected: bool) -> impl IntoView {
    // Define dynamic style for background color based on `selected`
    let background_color = if selected {
        "var(--good)"
    } else {
        "var(--ghostAZ)"
    };

    let styler_class = style! {
        "ChainButtonIcon",
        .chain-button-icon {
            position: absolute;
            top: 12px;
            left: 10px;
            width: 20px;
            height: 20px;
            border-radius: 10px;
            box-sizing: border-box;
            border: solid 3px var(--ghostZ);
        }
    };

    view! { class=styler_class,
        <div class="chain-button-icon" style=format!("background: {};", background_color) />
    }
}
