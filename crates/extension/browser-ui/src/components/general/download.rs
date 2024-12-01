use leptos::*;
use stylers::style;

#[component]
pub fn Download(
    #[prop(optional)] href: Option<String>,
    #[prop(optional)] target: Option<String>,
    children: Children,
) -> impl IntoView {
    // Set default href if none is provided
    let href = href.unwrap_or_else(|| "#".to_string());

    // Define the styles using `stylers`
    let styler_class = style! { "Download",
        .download {
            color: var(--good);
            height: 64px;
            width: 100%;
            font-weight: 700;
            display: flex;
            justify-content: center;
            align-items: center;
            text-transform: uppercase;
            cursor: pointer;
            font-size: 17px;
            letter-spacing: 1px;
            text-decoration: none; /* Ensure no underline */
        }

        .download * {
            pointer-events: none;
        }

        .download:visited {
            color: var(--good);
        }
    };

    view! { class=styler_class,
        <a class="download" href=href target=target>
            {children()}
        </a>
    }
}
