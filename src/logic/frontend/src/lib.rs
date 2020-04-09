use js_sys::{Function as JsFunction, TypeError};
use rustpython_vm as vm;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vm::obj::objcode::PyCodeRef;
use vm::obj::objfunction::PyFunctionRef;
use vm::pyobject::{ItemProtocol, PyValue};
use wasm_bindgen::prelude::*;

mod pyconvert;
mod stdlib;
use pyconvert::PyResultExt;

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
                &JsValue::NULL,
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
                .call1(&JsValue::NULL, &JsValue::from_serde(&turn_state).unwrap())
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
    log_callback: &JsFunction,
    turn_num: usize,
) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let vm = &vm::VirtualMachine::new(vm::PySettings {
        initialization_parameter: vm::InitParameter::InitializeInternal,
        ..Default::default()
    });

    let py_state: Rc<RefCell<logic::StateForRobotInput>> = Rc::default();
    let py_cur_team = Rc::new(Cell::new(logic::Team::Red));

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

        stdlib::add(&py_state, &py_cur_team, log_callback, vm);

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

    let run_team = |team, input: logic::RobotInput| -> Result<_, JsValue> {
        py_cur_team.set(team);
        *py_state.borrow_mut() = input.state;
        let state = py_state.borrow();

        let robot = match team {
            logic::Team::Red => &red,
            logic::Team::Blue => &blue,
        };

        let turn = vm.new_int(state.turn);

        let actions = state.teams[&team]
            .iter()
            .map(|id| -> Result<_, JsValue> {
                let obj = stdlib::Obj(state.objs[id].clone())
                    .into_ref(vm)
                    .into_object();
                let ret = robot.invoke(vec![turn.clone(), obj].into(), vm).to_js(vm)?;
                let action = ret
                    .payload::<stdlib::Action>()
                    .ok_or_else(|| TypeError::new("Robot did not return an action!"))?;
                Ok((*id, action.0))
            })
            .collect::<Result<_, _>>()?;

        Ok(logic::RobotOutput { actions })
    };

    let output = logic::run(
        run_team,
        |turn_state| {
            turn_callback
                .call1(&JsValue::NULL, &JsValue::from_serde(&turn_state).unwrap())
                .expect("Turn callback function failed");
        },
        turn_num,
    )?;

    Ok(JsValue::from_serde(&output).unwrap())
}
