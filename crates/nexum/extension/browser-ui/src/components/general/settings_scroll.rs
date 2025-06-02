use leptos::prelude::*;
use stylers::style;

#[component]
pub fn SettingsScroll(
    #[prop(optional, default = 0)] scroll_bar: i32,
    children: Children,
) -> impl IntoView {
    // Define styles with `stylers`
    let styler_class = style! { "SettingsScroll",
        .settings-scroll {
            overflow-x: hidden;
            overflow-y: scroll;
            box-sizing: border-box;
            max-height: 580px;
            background: var(--ghostY);
            margin: 10px;
            border-radius: 30px;
        }
    };

    // Dynamically set the margin-right style based on scroll_bar prop
    let dynamic_style = move || format!("margin-right: -{scroll_bar}px;");

    view! { class=styler_class,
        <div class="settings-scroll" style=dynamic_style()>
            {children()}
        </div>
    }
}
