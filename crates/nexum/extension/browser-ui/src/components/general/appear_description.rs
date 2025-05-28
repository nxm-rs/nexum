use leptos::prelude::*;
use stylers::style;

#[component]
pub fn AppearDescription(mm_appear: ReadSignal<bool>, children: Children) -> impl IntoView {
    // SVG paths based on the `mm_appear` prop
    let svg_path_mm = "M288 64C39.52 64 0 182.1 0 273.5C0 379.5 78.8 448 176 448c27.33 0 51.21-6.516 66.11-36.79l19.93-40.5C268.3 358.6 278.1 352.4 288 352.1c9.9 .3711 19.7 6.501 25.97 18.63l19.93 40.5C348.8 441.5 372.7 448 400 448c97.2 0 176-68.51 176-174.5C576 182.1 536.5 64 288 64zM160 320c-35.35 0-64-28.65-64-64s28.65-64 64-64c35.35 0 64 28.65 64 64S195.3 320 160 320zM416 320c-35.35 0-64-28.65-64-64s28.65-64 64-64c35.35 0 64 28.65 64 64S451.3 320 416 320z";
    let svg_path_frame = "M176 448C167.3 448 160 455.3 160 464V512h32v-48C192 455.3 184.8 448 176 448zM272 448c-8.75 0-16 7.25-16 16s7.25 16 16 16s16-7.25 16-16S280.8 448 272 448zM164 172l8.205 24.62c1.215 3.645 6.375 3.645 7.59 0L188 172l24.62-8.203c3.646-1.219 3.646-6.375 0-7.594L188 148L179.8 123.4c-1.215-3.648-6.375-3.648-7.59 0L164 148L139.4 156.2c-3.646 1.219-3.646 6.375 0 7.594L164 172zM336.1 315.4C304 338.6 265.1 352 224 352s-80.03-13.43-112.1-36.59C46.55 340.2 0 403.3 0 477.3C0 496.5 15.52 512 34.66 512H128v-64c0-17.75 14.25-32 32-32h128c17.75 0 32 14.25 32 32v64h93.34C432.5 512 448 496.5 448 477.3C448 403.3 401.5 340.2 336.1 315.4zM64 224h13.5C102.3 280.5 158.4 320 224 320s121.8-39.5 146.5-96H384c8.75 0 16-7.25 16-16v-96C400 103.3 392.8 96 384 96h-13.5C345.8 39.5 289.6 0 224 0S102.3 39.5 77.5 96H64C55.25 96 48 103.3 48 112v96C48 216.8 55.25 224 64 224zM104 136C104 113.9 125.5 96 152 96h144c26.5 0 48 17.88 48 40V160c0 53-43 96-96 96h-48c-53 0-96-43-96-96V136z";

    // Conditional attributes based on the `mm_appear` prop
    let (svg_viewbox, svg_path, fill_color) = match mm_appear.get() {
        true => ("0 0 576 512", svg_path_mm, "var(--mm)"),
        false => ("0 0 448 512", svg_path_frame, "var(--good)"),
    };

    // Styles
    let styler_class = style! { "AppearDescription",
        .appear-description {
            font-weight: 600;
            text-transform: uppercase;
            font-size: 14px;
            letter-spacing: 1px;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 50px;
        }
        .appear-description svg {
            height: 16px;
            margin-right: 8px;
        }
    };

    view! { class=styler_class,
        <div class="appear-description">
            <svg viewBox=svg_viewbox>
                <path fill=fill_color d=svg_path />
            </svg>
            {children()}
        </div>
    }
}
