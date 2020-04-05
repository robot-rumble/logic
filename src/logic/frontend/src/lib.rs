mod utils;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn main(
    run_team: &js_sys::Function,
    turn_callback: &js_sys::Function,
    finish_callback: &js_sys::Function,
    turn_num: usize,
) {
    utils::set_panic_hook();

    let run_team = move |team, robot_state| {
        run_team
            .call2(
                &JsValue::NULL,
                &JsValue::from_serde(&team).unwrap(),
                &JsValue::from_serde(&robot_state).unwrap(),
            )
            .expect("Code runner failed.")
            .into_serde()
            .expect("Code runner returned invalid object.")
    };

    logic::run(
        run_team,
        |turn_state| {
            turn_callback
                .call1(&JsValue::NULL, &JsValue::from_serde(&turn_state).unwrap())
                .expect("Turn callback function failed.");
        },
        |final_state| {
            finish_callback
                .call1(&JsValue::NULL, &JsValue::from_serde(&final_state).unwrap())
                .expect("Final callback function failed.");
        },
        turn_num,
    )
}
