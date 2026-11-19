use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::{Closure, wasm_bindgen};

use crate::App;

const GLOBAL: &str = "danfseWasm";

#[wasm_bindgen(start)]
pub fn iniciar() {
    let api = match App::new() {
        Ok(app) => montar_api(app),
        Err(e) => objeto(&[
            ("ready", JsValue::FALSE),
            ("error", JsValue::from_str(&e)),
            ("gerarPdf", JsValue::UNDEFINED),
        ]),
    };
    // sem global não há como reportar nada ao chamador
    let _ = Reflect::set(&js_sys::global(), &JsValue::from_str(GLOBAL), &api);
}

fn montar_api(app: App) -> JsValue {
    let gerar = Closure::<dyn Fn(JsValue) -> JsValue>::new(move |arg: JsValue| {
        let entrada = (!arg.is_undefined()).then(|| texto_go(&arg));
        match app.gerar_pdf(entrada.as_deref()) {
            Ok(resp) => objeto(&[
                ("pdf", JsValue::from_str(&resp.pdf)),
                ("size", JsValue::from_f64(resp.size as f64)),
            ]),
            Err(e) => objeto(&[("error", JsValue::from_str(&e))]),
        }
    });
    let funcao: Function = gerar.as_ref().clone().into();
    // a página inteira usa a função; nunca é liberada, como a js.Func do Go
    gerar.forget();
    objeto(&[
        ("ready", JsValue::TRUE),
        ("error", JsValue::from_str("")),
        ("gerarPdf", funcao.into()),
    ])
}

/// `js.Value.String()` do Go: não-strings viram `<tipo>`, que o parser JSON
/// rejeita com a mesma mensagem
fn texto_go(v: &JsValue) -> String {
    if let Some(s) = v.as_string() {
        s
    } else if v.is_null() {
        "<null>".into()
    } else if let Some(b) = v.as_bool() {
        format!("<boolean: {b}>")
    } else if let Some(n) = v.as_f64() {
        format!("<number: {n}>")
    } else if v.is_symbol() {
        "<symbol>".into()
    } else if v.is_function() {
        "<function>".into()
    } else {
        "<object>".into()
    }
}

fn objeto(campos: &[(&str, JsValue)]) -> JsValue {
    let obj = Object::new();
    for (chave, valor) in campos {
        let _ = Reflect::set(&obj, &JsValue::from_str(chave), valor);
    }
    obj.into()
}
