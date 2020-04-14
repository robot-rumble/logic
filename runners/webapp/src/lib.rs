use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn run(
    code1: &str,
    code2: &str,
    turn_callback: &JsFunction,
    turn_num: usize,
) -> Result<JsValue, JsValue> {
    let mut lang_runners = HashMap::new();
    lang_runners.insert(logic::Team::Red, pyrunner::init(code1));
    lang_runners.insert(logic::Team::Blue, pyrunner::init(code1));

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
    )?;

    Ok(JsValue::from_serde(&output).unwrap())
}
