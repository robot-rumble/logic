use rustpython_vm::obj::objcode::PyCodeRef;
use rustpython_vm::obj::objstr::PyStringRef;
use rustpython_vm::py_compile_bytecode;
use rustpython_vm::py_serde;
use rustpython_vm::pyobject::{ItemProtocol, PyResult};
use rustpython_vm::scope::Scope;
use rustpython_vm::VirtualMachine;

pub fn run_python(
    code1: PyCodeRef,
    code2: PyCodeRef,
    turn_callback: impl FnMut(&logic::TurnState),
    log_callback: Option<impl Fn(&str) + 'static>,
    turn_num: usize,
    vm: &VirtualMachine,
) -> PyResult<logic::MainOutput> {
    let (_, frozen): (String, _) =
        py_compile_bytecode!(file = "stdlib.py", module_name = "<stdlib>")
            .into_iter()
            .next()
            .unwrap();
    let stdlib_code = vm.ctx.new_code_object(frozen.code);

    let create_main_fn = |code: PyCodeRef| -> PyResult {
        let attrs = vm.ctx.new_dict();
        attrs.set_item("__name__", vm.new_str("<robot>".to_owned()), vm)?;

        vm.run_code_obj(
            stdlib_code.clone(),
            Scope::with_builtins(None, attrs.clone(), vm),
        )?;

        vm.run_code_obj(code, Scope::with_builtins(None, attrs.clone(), vm))?;

        let robot = attrs
            .get_item_option("_main", vm)?
            .ok_or_else(|| vm.new_type_error("you must define a 'robot' function".to_owned()))?;

        Ok(robot)
    };

    let red = create_main_fn(code1)?;
    let blue = create_main_fn(code2)?;

    let log_func =
        log_callback.map(|log| vm.ctx.new_function(move |s: PyStringRef| log(s.as_str())));

    let run_team = |team, input: logic::RobotInput| -> PyResult<_> {
        let robot = match team {
            logic::Team::Red => &red,
            logic::Team::Blue => &blue,
        };

        // TODO(noah): impl Serializer, Deserializer in py_serde so this isn't necessary
        let state = serde_cbor::to_vec(&input).unwrap();
        let mut state_deserializer = serde_cbor::Deserializer::from_slice(&state);

        let mut args = vec![py_serde::deserialize(vm, &mut state_deserializer).unwrap()];
        if let Some(ref log) = log_func {
            args.push(log.clone())
        }

        let ret = vm.invoke(&robot, args)?;
        let actions = serde_cbor::value::to_value(&py_serde::PyObjectSerializer::new(vm, &ret))
            .and_then(serde_cbor::value::from_value)
            .map_err(|e| vm.new_type_error(e.to_string()))?;

        Ok(logic::RobotOutput { actions })
    };

    logic::run(run_team, turn_callback, turn_num)
}
