use futures::Future;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement, Response, Window};

pub fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or(JsValue::from("No Window Found"))
}

pub fn document() -> Result<Document, JsValue> {
    window()?
        .document()
        .ok_or(JsValue::from("No Document Found"))
}

pub fn canvas() -> Result<HtmlCanvasElement, JsValue> {
    document()?
        .get_element_by_id("canvas")
        .ok_or(JsValue::from("No Canvas Element found with ID 'canvas'"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|element| JsValue::from(element))
}

pub fn context() -> Result<CanvasRenderingContext2d, JsValue> {
    canvas()?
        .get_context("2d")?
        .ok_or(JsValue::from("No 2d context found"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|element| JsValue::from(element))
}

pub fn spawn_local<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

pub async fn fetch_with_str(resource: &str) -> Result<JsValue, JsValue> {
    JsFuture::from(window()?.fetch_with_str(resource)).await
}

pub async fn fetch_json(json_path: &str) -> Result<JsValue, JsValue> {
    let resp_value = fetch_with_str(json_path).await?;
    let resp: Response = resp_value.dyn_into()?;

    JsFuture::from(resp.json()?).await
}

pub fn create_one_time_closure<F>(f: F) -> Closure<dyn FnMut()>
where
    F: Fn() + 'static,
{
    Closure::once(Box::new(f))
}

pub fn create_one_time_closure_with_err<F>(f: F) -> Closure<dyn FnMut(JsValue)>
where
    F: Fn(JsValue) + 'static,
{
    Closure::once(Box::new(f))
}
