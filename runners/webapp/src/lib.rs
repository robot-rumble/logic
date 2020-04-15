use js_sys::Function as JsFunction;
use maplit::hashmap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn run(code1: &str, code2: &str, turn_callback: &JsFunction, turn_num: usize) -> JsValue {
    let lang_runners = hashmap! {
        // TODO: what to do about this?
        logic::Team::Red => pyrunner::init(code1).unwrap(),
        logic::Team::Blue => pyrunner::init(code2).unwrap(),
    };

    let output = logic::run(
        lang_runners,
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
