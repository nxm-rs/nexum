use leptos::prelude::*;
use stylers::style;

#[component]
pub fn AppearToggle(
    #[prop(optional)] on_click: Option<Box<dyn Fn()>>,
    children: Children,
) -> impl IntoView {
    let styler_class = style! {"AppearToggle",
        .appear-toggle {
            position: relative;
            height: 32px;
            font-weight: 600;
            display: flex;
            justify-content: center;
            align-items: center;
            text-transform: uppercase;
            cursor: pointer;
            font-size: 12px;
            overflow: hidden;
            letter-spacing: 1px;
        }
    };

    // Define an event handler that triggers the callback if provided
    let handle_click = move |_| {
        if let Some(on_click) = &on_click {
            on_click();
        }
    };

    view! { class=styler_class,
        <div class="appear-toggle" on:click=handle_click>
            {children()}
        </div>
    }
}
