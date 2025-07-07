use leptos::prelude::*;
use leptos::web_sys::KeyboardEvent;
use std::rc::Rc;
use stylers::style;

#[component]
pub fn Value(
    #[prop(optional)] pointer_events: bool,
    #[prop(optional)] transparent: bool,
    #[prop(optional)] on_interact: Option<Box<dyn Fn()>>,
    #[prop(optional)] style: Option<String>,
    children: Children,
) -> impl IntoView {
    // Wrap `on_click` in an `Rc` so it can be shared between multiple closures
    let on_interact = Rc::new(on_interact);

    // Define the main styles with classes
    let styler_class = style! { "Value",
        .cluster-value {
            flex-grow: 1;
            display: flex;
            justify-content: center;
            align-items: center;
            border-radius: 8px;
            margin-right: 3px;
            font-size: 14px;
            background: var(--ghostA);
            box-shadow: 0px 1px 2px var(--ghostX);
            border-bottom: 2px solid var(--ghostZ);
            overflow: hidden;
            margin-top: 1px;
            transition: all linear 0.1s;
            transform: translate3d(0, 0, 0);
            font-family: "MainFont";
        }
        .clickable {
            cursor: pointer;
            margin-bottom: 0px;
            position: relative;
            z-index: 3;
        }
        .clickable:hover {
            background: var(--ghostB);
            transform: translateY(-1px);
            border-bottom: 2px solid var(--ghostZ);
            box-shadow: 0px 4px 30px -8px var(--ghostX);
            z-index: 300000;
        }
        .clickable:active {
            background: var(--ghostB);
            transform: translateY(0px);
            box-shadow: 0px 2px 4px var(--ghostX);
        }
        .transparent {
            background: transparent;
            box-shadow: none;
            border-bottom: 2px solid transparent;
        }
        .pointer-events {
            pointer-events: auto;
        }
    };

    // Define event handlers for click and keydown events
    let handle_click = {
        let on_click = Rc::clone(&on_interact);
        move |_| {
            if let Some(ref on_click) = *on_click {
                on_click();
            }
        }
    };

    let handle_keydown = {
        let on_click = Rc::clone(&on_interact);
        move |event: KeyboardEvent| {
            if (event.key() == "Enter" || event.key() == " ") && on_click.is_some()
                && let Some(ref on_click) = *on_click {
                    on_click();
                }
        }
    };

    // Combine custom inline styles from the `custom_style` prop if provided
    let combined_style = move || style.clone().unwrap_or_default();

    view! { class=styler_class,
        <div
            class="cluster-value"
            class:clickable=on_interact.is_some() || pointer_events
            class:transparent=transparent
            class:pointer-events=pointer_events
            on:click=handle_click
            on:keydown=handle_keydown
            role="button"
            tabindex="0"
            style=combined_style()
        >
            {children()}
        </div>
    }
}
