extern crate rustpython_vm as vm;
use vm::obj::objcode::PyCodeRef;
use vm::obj::objfunction::PyFunctionRef;
use vm::pyobject::ItemProtocol;

use js_sys::{Function as JsFunction, TypeError};
use wasm_bindgen::prelude::*;

mod pyconvert;
use pyconvert::PyResultExt;

#[wasm_bindgen]
pub fn run_logic(
    run_team: &JsFunction,
    turn_callback: &JsFunction,
    finish_callback: &JsFunction,
    turn_num: usize,
) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let run_team = move |team, robot_state| {
        run_team
            .call2(
                &JsValue::NULL,
                &JsValue::from_serde(&team).unwrap(),
                &JsValue::from_serde(&robot_state).unwrap(),
            )?
            .into_serde()
            .map_err(|e| TypeError::new(&format!("runner returned invalid object: {}", e)).into())
    };

    logic::run(
        run_team,
        |turn_state| {
            turn_callback
                .call1(&JsValue::NULL, &JsValue::from_serde(&turn_state).unwrap())
                .expect("Turn callback function failed");
        },
        |final_state| {
            finish_callback
                .call1(&JsValue::NULL, &JsValue::from_serde(&final_state).unwrap())
                .expect("Final callback function failed");
        },
        turn_num,
    )
}

#[wasm_bindgen]
pub fn run_rustpython(
    code1: &str,
    code2: &str,
    turn_callback: &JsFunction,
    finish_callback: &JsFunction,
    turn_num: usize,
) -> Result<(), JsValue> {
    let vm = &vm::VirtualMachine::new(vm::PySettings {
        initialization_parameter: vm::InitParameter::InitializeInternal,
        ..Default::default()
    });

    let create_robot_fn = |code| -> Result<PyFunctionRef, JsValue> {
        let code = vm
            .compile(
                code,
                rustpython_compiler::compile::Mode::Exec,
                "<robot>".to_owned(),
            )
            .map_err(pyconvert::syntax_err)?;

        let attrs = vm.ctx.new_dict();
        attrs
            .set_item("__name__", vm.new_str("<robot>".to_owned()), vm)
            .to_js(vm)?;

        // Execute main code in module:
        vm.run_code_obj(
            code.clone(),
            vm::scope::Scope::with_builtins(None, attrs.clone(), vm),
        )
        .to_js(vm)?;

        let robot = attrs
            .get_item_option("robot", vm)
            .to_js(vm)?
            .ok_or_else(|| TypeError::new("you must define a 'robot' function"))?;

        let robot: PyFunctionRef = robot
            .downcast()
            .map_err(|_| TypeError::new("'robot' should be a function"))?;

        // TODO(noah): add a .code() getter to PyFunction
        let code: PyCodeRef = vm
            .get_attribute(robot.as_object().clone(), "__code__")
            .unwrap()
            .downcast()
            .unwrap();

        if code.arg_names.len() != 2 {
            let msg =
                "Your 'robot' function must accept two values: the current turn and robot details.";
            return Err(TypeError::new(msg).into());
        }

        Ok(robot)
    };

    let red = create_robot_fn(code1)?;
    let blue = create_robot_fn(code2)?;

    let run_team = |team, input: logic::RobotInput| {
        let robot = match team {
            logic::Team::Red => &red,
            logic::Team::Blue => &blue,
        };

        let turn = vm.new_int(input.state.turn);

        let actions = input.state.teams[&team]
            .iter()
            .map(|id| {
                // TODO(noah): fix rustpython so we don't have to do this dance of struct <> serde json value <> pyobject
                let obj = serde_json::to_value(&input.state.objs[id]).unwrap();
                let args = vec![turn.clone(), vm::py_serde::deserialize(vm, obj).unwrap()];
                let ret = robot.invoke(args.into(), vm).to_js(vm)?;
                if vm.is_none(&ret) {
                    return Err(TypeError::new("Robot did not return an action!"));
                }
                let ret = vm::py_serde::serialize(vm, &ret, serde_json::value::Serializer)
                    .map_err(|e| TypeError::new(&e.to_string()))?;
                let action = serde_json::from_value(ret).map_err(|e| {
                    TypeError::new(&format!("invalid action returned from robot: {}", e))
                })?;
                Ok((*id, action))
            })
            .collect::<Result<_, _>>()?;

        Ok(logic::RobotOutput { actions })
    };

    logic::run(
        run_team,
        |turn_state| {
            turn_callback
                .call1(&JsValue::NULL, &JsValue::from_serde(&turn_state).unwrap())
                .expect("Turn callback function failed");
        },
        |final_state| {
            finish_callback
                .call1(&JsValue::NULL, &JsValue::from_serde(&final_state).unwrap())
                .expect("Final callback function failed");
        },
        turn_num,
    )
}
