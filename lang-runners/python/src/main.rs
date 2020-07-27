use rustpython_vm::obj::objdict::PyDictRef;
use rustpython_vm::py_compile_bytecode;
use rustpython_vm::py_serde;
use rustpython_vm::pyobject::{ItemProtocol, PyObjectRef};
use rustpython_vm::scope::Scope;
use rustpython_vm::{InitParameter, PySettings, VirtualMachine};

use logic::{ProgramError, ProgramInput, ProgramResult};
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
    vm.unwrap_pyresult(attrs.set_item("__name__", vm.new_str("<robot>".to_owned()), vm));
    vm.unwrap_pyresult(vm.run_code_obj(
        vm.ctx.new_code_object(CODE.clone()),
        Scope::with_builtins(None, attrs.clone(), vm),
    ));
    attrs
}

fn serde_to_py<T: serde::Serialize>(
    s: &T,
    vm: &VirtualMachine,
) -> Result<PyObjectRef, ProgramError> {
    let val = serde_json::to_value(s)?;
    let py = py_serde::deserialize(vm, val)?;
    Ok(py)
}

fn py_to_serde<T: serde::de::DeserializeOwned>(
    py: &PyObjectRef,
    vm: &VirtualMachine,
) -> Result<T, ProgramError> {
    let val = py_serde::serialize(vm, py, serde_json::value::Serializer)?;
    let out = serde_json::from_value(val)?;
    Ok(out)
}

fn invoke_main(main: &PyObjectRef, input: &ProgramInput, vm: &VirtualMachine) -> ProgramResult {
    let ret = vm
        .invoke(main, vec![serde_to_py(&input, vm)?])
        .map_err(|_| ProgramError::InternalError)?;
    py_to_serde(&ret, vm).and_then(|r| r)
}

pub fn init(code: &str) -> Result<impl FnMut(ProgramInput) -> ProgramResult, ProgramError> {
    let vm = VirtualMachine::new(PySettings {
        initialization_parameter: InitParameter::InitializeInternal,
        ..Default::default()
    });
    let code = vm
        .compile(
            code,
            rustpython_compiler::compile::Mode::Exec,
            "<robot>".to_owned(),
        )
        .map_err(|err| {
            ProgramError::InitError(logic::Error {
                message: err.to_string(),
                loc: Some(logic::ErrorLoc {
                    start: (err.location.row(), Some(err.location.column())),
                    end: None,
                }),
            })
        })?;

    let attrs = setup_scope(&vm);
    let formatexc = vm.unwrap_pyresult(attrs.get_item("__format_err", &vm));

    let make_main = || {
        vm.run_code_obj(code, Scope::with_builtins(None, attrs.clone(), &vm))?;
        attrs.get_item("__main", &vm).map_err(|_| {
            vm.new_type_error(
                "you must **not** delete the `__main` function, c'mon, dude".to_owned(),
            )
        })
    };
    let main = match make_main() {
        Ok(f) => f,
        Err(exc) => {
            // if setup errors, try to format the error, and just return an InternalError if it
            // doesn't work
            let exc = vm
                .invoke(&formatexc, vec![exc.into_object()])
                .map_err(|_| ProgramError::InternalError)?;
            let err = py_to_serde(&exc, &vm).map_err(|_| ProgramError::InternalError)?;
            return Err(ProgramError::InitError(err));
        }
    };

    Ok(move |input| invoke_main(&main, &input, &vm))
}

include!("../../lang-common.rs");

lang_runner!(init);
