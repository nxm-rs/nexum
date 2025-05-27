use leptos::*;
use stylers::style;

#[component]
pub fn LogoWrap(src: String, alt: String) -> impl IntoView {
    // Define styles with `stylers`
    let styler_class = style! { "LogoWrap",
        .logo-wrap {
            width: 80px;
            height: 50px;
            display: flex;
            justify-content: center;
            align-items: center;
            box-sizing: border-box;
        }

        .logo-wrap img {
            height: 20px;
        }
    };

    view! { class=styler_class,
        <div class="logo-wrap">
            <img src=src alt=alt />
        </div>
    }
}
