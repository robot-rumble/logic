use js_sys::{Function as JsFunction, TypeError};
use rustpython_vm as vm;
use wasm_bindgen::prelude::*;

mod pyconvert;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn run_logic(
    run_team: &JsFunction,
    turn_callback: &JsFunction,
    turn_num: usize,
) -> Result<JsValue, JsValue> {
    let run_team = move |team, robot_state| -> Result<_, JsValue> {
        run_team
            .call2(
                &JsValue::UNDEFINED,
                &JsValue::from_serde(&team).unwrap(),
                &JsValue::from_serde(&robot_state).unwrap(),
            )?
            .into_serde()
            .map_err(|e| TypeError::new(&format!("runner returned invalid object: {}", e)).into())
    };

    let output = logic::run(
        run_team,
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

#[wasm_bindgen]
pub fn run_rustpython(
    code1: &str,
    code2: &str,
    turn_callback: &JsFunction,
    log_callback: JsFunction,
    turn_num: usize,
) -> Result<JsValue, JsValue> {
    let vm = &vm::VirtualMachine::new(vm::PySettings {
        initialization_parameter: vm::InitParameter::InitializeInternal,
        ..Default::default()
    });
    let compile = |source| {
        vm.compile(
            source,
            rustpython_compiler::compile::Mode::Exec,
            "<robot>".to_owned(),
        )
        .map_err(pyconvert::syntax_err)
    };
    pyrunner::run_python(
        compile(code1)?,
        compile(code2)?,
        |turn_state| {
            turn_callback
                .call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_serde(&turn_state).unwrap(),
                )
                .expect("Turn callback function failed");
        },
        move |s| {
            log_callback
                .call1(&JsValue::UNDEFINED, &s.into())
                .expect("log callback failed");
        },
        turn_num,
        vm,
    )
    .map(|output| JsValue::from_serde(&output).unwrap())
    .map_err(|py_err| pyconvert::py_err_to_js_err(vm, &py_err))
}
