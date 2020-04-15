use js_sys::Function as JsFunction;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn info(s: &str);
}

#[wasm_bindgen]
pub fn run(code1: &str, code2: &str, turn_callback: &JsFunction, turn_num: usize) -> JsValue {
    info("starting");
    let r1 = pyrunner::init(code1);
    info("done with first");
    let r2 = pyrunner::init(code2);
    info("after");
    let output = logic::run(
        r1,
        r2,
        |turn_state| {
            turn_callback
                .call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_serde(&turn_state).unwrap(),
                )
                .expect("Turn callback function failed");
        },
        turn_num,
    );

    JsValue::from_serde(&output).unwrap()
}
