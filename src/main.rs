mod helpers;

use yew::{html, Component, Context, Html, events::Event, TargetCast};
use web_sys::{HtmlInputElement, File};
use wasm_bindgen_futures::spawn_local;

#[derive(Debug)]
pub enum Msg {
    Files(Event),
    Receive(Event),
}

#[derive(Debug)]
pub struct App {
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Files(event) => {
                let mut selected_files = Vec::new();
                let input: HtmlInputElement = event.target_unchecked_into();
                if let Some(files) = input.files() {
                    let files = js_sys::try_iter(&files)
                        .unwrap()
                        .unwrap()
                        .map(|v| File::from(v.unwrap()));
                    selected_files.extend(files);
                }
                for file in selected_files.into_iter() {
                    spawn_local(async move {
                        helpers::send(file).await;
                    });
                }
                false
            }
            Msg::Receive(event) => {
                let input: HtmlInputElement = event.target_unchecked_into();
                let code = input.value();
                log::debug!("Requesting code: {}", code);
                spawn_local(async move {
                    helpers::receive(code).await;
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <div>
                    <input type="file" multiple={true} onchange={ctx.link().callback(|e| Msg::Files(e))}/>
                </div>
                <div>
                    <input type="text" placeholder={"Code"} onchange={ctx.link().callback(|e| Msg::Receive(e))}/>
                </div>
            </div>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}