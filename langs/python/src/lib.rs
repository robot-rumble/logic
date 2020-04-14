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
use logic::ProgramOutput;

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

fn create_main(code: PyCodeRef, attrs: PyDictRef, vm: &VirtualMachine) -> PyResult {
    vm.run_code_obj(code, Scope::with_builtins(None, attrs.clone(), vm))?;

    let robot = attrs
        .get_item_option("_main", vm)?
        .ok_or_else(|| vm.new_type_error("you must define a 'robot' function".to_owned()))?;

    Ok(robot)
}

fn invoke_main(
    main: &PyObjectRef,
    input: &logic::ProgramInput,
    log_func: Option<PyObjectRef>,
    vm: &VirtualMachine,
) -> logic::ProgramResult {
    // TODO(noah): impl Serializer, Deserializer in py_serde so this isn't necessary
    let state = serde_cbor::to_vec(&input).unwrap();
    let mut state_deserializer = serde_cbor::Deserializer::from_slice(&state);
    let state = py_serde::deserialize(vm, &mut state_deserializer).unwrap();

    let args = std::iter::once(state).chain(log_func).collect::<Vec<_>>();
    let ret = vm.invoke(main, args)?;

    serde_cbor::value::to_value(&py_serde::PyObjectSerializer::new(vm, &ret)).and_then(serde_cbor::value::from_value)
}

pub fn init(code: &str) -> RunF
where
    RunF: FnMut(logic::ProgramInput) -> Result<logic::ProgramOutput, Err>,
{
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

    let attrs = setup_scope(vm);
    let main = create_main(compile(code), attrs, vm)?;

    |input: logic::ProgramInput| -> logic::ProgramOutput {
        let mut logs = Vec::new();
        let log_func = vm.ctx.new_function(move |s: PyStringRef| logs.push(s.as_str().into()));
        let robot_outputs = invoke_main(&main, &input, log_func, vm);

        logic::ProgramOutput { robot_outputs, logs }
    }
}

// pub fn make_secure_python_runf<E>(
//     code: PyCodeRef,
//     vm: VirtualMachine,
//     log_callback: Option<impl Fn(&str) + 'static>,
//     map_error: impl Fn(PyBaseExceptionRef, &VirtualMachine) -> E,
// ) -> PyResult<impl FnMut(logic::ProgramInput) -> Result<logic::ProgramOutput, E>> {
//     let log_func =
//         log_callback.map(|log| vm.ctx.new_function(move |s: PyStringRef| log(s.as_str())));
//
//     let main = create_main(code, setup_scope(&vm), &vm)?;
//
//     Ok(move |input| {
//         invoke_main(&main, &input, log_func.clone(), &vm).map_err(|e| map_error(e, &vm))
//     })
// }
