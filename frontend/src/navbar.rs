use yew::{function_component, html, Children, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct NavBarProps {
    #[prop_or_default]
    pub user: Option<String>,
    pub children: Children,
}

#[function_component(NavBar)]
pub fn navbar(props: &NavBarProps) -> Html {
    html! {
        <>
            <div class="bg-gray-100">
                <div class="container mx-auto navbar">
                    <div class="flex-1">
                        <h1 class="font-bold normal-case text-xl">{ "nanum" }</h1>
                    </div>
                    if let Some(user) = &props.user {
                        <div class="flex-none">
                            <span>{ user }</span>
                        </div>
                    } else {
                        <></>
                    }
                </div>
            </div>
            <div class="h-[calc(100vh-4rem)] w-full bg-gray-200 flex items-center justify-center">
                { for props.children.iter() }
            </div>
        </>
    }
}
