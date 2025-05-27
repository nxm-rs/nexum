use leptos::prelude::*;
use stylers::style;

#[component]
pub fn AddressRecipient(children: Children) -> impl IntoView {
    let styler_class = style! {"AddressRecipient",
        .cluster-address-recipient {
            font-size: 16px;
            font-weight: 300;
            font-family: "FiraCode";
            display: flex;
            justify-content: center;
            align-items: center;
            pointer-events: none;
        }
    };

    view! { class=styler_class, <div class="cluster-address-recipient">{children()}</div> }
}
