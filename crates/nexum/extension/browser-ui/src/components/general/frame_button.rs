use leptos::prelude::*;
use stylers::style;

#[component]
pub fn FrameButton(children: Children) -> impl IntoView {
    let styler_class = style! {"FrameButton",
        .frame-button {
            width: 140px;
            height: 30px;
            display: flex;
            justify-content: center;
            align-items: center;
            box-sizing: border-box;
            font-size: 16px;
            font-weight: 400;
        }
    };

    view! { class=styler_class, <div class="frame-button">{children()}</div> }
}
