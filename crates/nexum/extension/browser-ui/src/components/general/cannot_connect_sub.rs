use leptos::prelude::*;
use stylers::style;

#[component]
pub fn CannotConnectSub(children: Children) -> impl IntoView {
    let styler_class = style! {"CannotConnectSub",
        .cannot-connect-sub {
            padding: 0px 32px;
            display: flex;
            justify-content: center;
            align-items: center;
            font-size: 14px;
            flex-direction: column;
        }
    };

    view! { class=styler_class, <div class="cannot-connect-sub">{children()}</div> }
}
