use js_sys::Function as JsFunction;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn run(code1: &str, code2: &str, turn_callback: &JsFunction, turn_num: usize) -> JsValue {
    let output = logic::run(
        pyrunner::init(code1),
        pyrunner::init(code2),
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

    let output = futures::executor::block_on(output);

    JsValue::from_serde(&output).unwrap()
}
