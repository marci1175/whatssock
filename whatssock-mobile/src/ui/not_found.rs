use dioxus::prelude::*;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        div {
            id: "not_found_page",
            {
                format!(r#"Path "{:?}" is not found."#, segments)
            }
        }
    }
}
