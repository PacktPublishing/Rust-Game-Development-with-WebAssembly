use futures::Future;
use js_sys::Function;
use wasm_bindgen::{
    closure::WasmClosure, closure::WasmClosureFnOnce, prelude::Closure, JsCast, JsValue,
};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasRenderingContext2d, Document, HtmlCanvasElement, HtmlImageElement, Response, Window,
};

// Straight taken from https://rustwasm.github.io/book/game-of-life/debugging.html
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or_else(|| JsValue::from("No Window Found"))
}

pub fn document() -> Result<Document, JsValue> {
    window()?
        .document()
        .ok_or_else(|| JsValue::from("No Document Found"))
}

pub fn canvas() -> Result<HtmlCanvasElement, JsValue> {
    document()?
        .get_element_by_id("canvas")
        .ok_or_else(|| JsValue::from("No Canvas Element found with ID 'canvas'"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|element| JsValue::from(element))
}

pub fn context() -> Result<CanvasRenderingContext2d, JsValue> {
    canvas()?
        .get_context("2d")?
        .ok_or_else(|| JsValue::from("No 2d context found"))?
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

pub fn new_image() -> Result<HtmlImageElement, JsValue> {
    HtmlImageElement::new()
}

pub fn request_animation_frame(callback: &Function) -> Result<i32, JsValue> {
    window()?.request_animation_frame(callback)
}

pub fn closure_once<F, A, R>(fn_once: F) -> Closure<F::FnMut>
where
    F: 'static + WasmClosureFnOnce<A, R>,
{
    Closure::once(fn_once)
}

pub fn closure_wrap<T: WasmClosure + ?Sized>(data: Box<T>) -> Closure<T> {
    Closure::wrap(data)
}

pub fn now() -> Result<f64, JsValue> {
    Ok(window()?
        .performance()
        .ok_or_else(|| JsValue::from("Performance object not found"))?
        .now())
}
