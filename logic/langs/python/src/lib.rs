use rustpython_vm::exceptions::PyBaseExceptionRef;
use rustpython_vm::obj::objcode::PyCodeRef;
use rustpython_vm::obj::objdict::PyDictRef;
use rustpython_vm::obj::objstr::PyStringRef;
use rustpython_vm::py_compile_bytecode;
use rustpython_vm::py_serde;
use rustpython_vm::pyobject::{ItemProtocol, PyObjectRef, PyResult, PyValue};
use rustpython_vm::scope::Scope;
use rustpython_vm::VirtualMachine;

use once_cell::sync::Lazy;

fn setup_scope(vm: &VirtualMachine) -> PyDictRef {
    static CODE: Lazy<rustpython_vm::bytecode::CodeObject> = Lazy::new(|| {
        let (_, frozen): (String, _) =
            py_compile_bytecode!(file = "stdlib.py", module_name = "<stdlib>")
                .into_iter()
                .next()
                .unwrap();
        frozen.code
    });

    let attrs = vm.ctx.new_dict();
    attrs
        .set_item("__name__", vm.new_str("<robot>".to_owned()), vm)
        .unwrap();
    vm.run_code_obj(
        vm.ctx.new_code_object(CODE.clone()),
        Scope::with_builtins(None, attrs.clone(), vm),
    )
    .unwrap();
    attrs
}

fn invoke_main(
    main: &PyObjectRef,
    input: &logic::RobotInput,
    log_func: Option<PyObjectRef>,
    vm: &VirtualMachine,
) -> PyResult<logic::RobotOutput> {
    // TODO(noah): impl Serializer, Deserializer in py_serde so this isn't necessary
    let state = serde_cbor::to_vec(&input).unwrap();
    let mut state_deserializer = serde_cbor::Deserializer::from_slice(&state);
    let state = py_serde::deserialize(vm, &mut state_deserializer).unwrap();

    let args = std::iter::once(state).chain(log_func).collect::<Vec<_>>();
    let ret = vm.invoke(main, args)?;

    let actions = serde_cbor::value::to_value(&py_serde::PyObjectSerializer::new(vm, &ret))
        .and_then(serde_cbor::value::from_value)
        .map_err(|e| vm.new_type_error(e.to_string()))?;

    Ok(logic::RobotOutput { actions })
}

fn create_main(code: PyCodeRef, attrs: PyDictRef, vm: &VirtualMachine) -> PyResult {
    vm.run_code_obj(code, Scope::with_builtins(None, attrs.clone(), vm))?;

    let robot = attrs
        .get_item_option("_main", vm)?
        .ok_or_else(|| vm.new_type_error("you must define a 'robot' function".to_owned()))?;

    Ok(robot)
}

pub fn run_python_insecure(
    code1: PyCodeRef,
    code2: PyCodeRef,
    turn_callback: impl FnMut(&logic::TurnState),
    log_callback: Option<impl Fn(&str) + 'static>,
    turn_num: usize,
    vm: &VirtualMachine,
) -> PyResult<logic::MainOutput> {
    let attrs = setup_scope(vm);

    let red = create_main(code1, attrs.clone().copy().into_ref(vm), vm)?;
    let blue = create_main(code2, attrs, vm)?;

    let log_func =
        log_callback.map(|log| vm.ctx.new_function(move |s: PyStringRef| log(s.as_str())));

    let run_team = |input: logic::RobotInput| -> PyResult<_> {
        let robot = match input.team {
            logic::Team::Red => &red,
            logic::Team::Blue => &blue,
        };

        invoke_main(robot, &input, log_func.clone(), vm)
    };

    logic::run(run_team, turn_callback, turn_num)
}

pub fn make_secure_python_runf<E>(
    code: PyCodeRef,
    vm: VirtualMachine,
    log_callback: Option<impl Fn(&str) + 'static>,
    map_error: impl Fn(PyBaseExceptionRef, &VirtualMachine) -> E,
) -> PyResult<impl FnMut(logic::RobotInput) -> Result<logic::RobotOutput, E>> {
    let log_func =
        log_callback.map(|log| vm.ctx.new_function(move |s: PyStringRef| log(s.as_str())));

    let main = create_main(code, setup_scope(&vm), &vm)?;

    Ok(move |input| {
        invoke_main(&main, &input, log_func.clone(), &vm).map_err(|e| map_error(e, &vm))
    })
}
