use chrome_sys::tabs;
use leptos::*;
use stylers::style;

#[component]
pub fn CurrentOriginTitle(tab: ReadSignal<Option<tabs::Info>>) -> impl IntoView {
    let styler_class = style! {"CurrentOriginTitle",
        .current-origin-title {
            position: relative;
            display: flex;
            justify-content: center;
            align-items: center;
            font-size: 15px;
            height: 44px;
            padding-top: 8px;
            margin-top: 0px;
            font-weight: 400;
        }

        .current-origin-title svg {
            position: relative;
            top: 1px;
            margin-right: 10px;
            height: 15px;
        }
    };
    view! { class=styler_class,
        <div class="current-origin-title">
            <svg viewBox="0 0 512 512">
                <path
                    fill="currentColor"
                    d="M448 32C483.3 32 512 60.65 512 96V416C512 451.3 483.3 480 448 480H64C28.65 480 0 451.3 0 416V96C0 60.65 28.65 32 64 32H448zM96 96C78.33 96 64 110.3 64 128C64 145.7 78.33 160 96 160H416C433.7 160 448 145.7 448 128C448 110.3 433.7 96 416 96H96z"
                />
            </svg>
            <span>{tab.get().map(|tab| tab.url).unwrap_or_default()}</span>
        </div>
    }
}
