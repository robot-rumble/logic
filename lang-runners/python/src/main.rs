use rustpython_vm::builtins::PyDictRef;
use rustpython_vm::py_compile;
use rustpython_vm::py_serde;
use rustpython_vm::pyobject::{ItemProtocol, PyObjectRef, PyResult};
use rustpython_vm::scope::Scope;
use rustpython_vm::{InitParameter, Interpreter, PySettings, VirtualMachine};

use logic::{ProgramError, ProgramResult};

fn setup_scope(vm: &VirtualMachine) -> PyDictRef {
    let code = vm.new_code_object(py_compile!(
        file = "stdlib/rumblelib.py",
        module_name = "rumblelib"
    ));

    let attrs = vm.ctx.new_dict();
    let run = || -> PyResult<()> {
        attrs.set_item("__name__", vm.ctx.new_str("<robot>".to_owned()), vm)?;
        vm.run_code_obj(code, Scope::with_builtins(None, attrs.clone(), vm))?;
        let sys_modules: PyDictRef = vm
            .get_attribute(vm.sys_module.clone(), "modules")?
            .downcast()
            .ok()
            .expect("sys.modules should be dict");
        sys_modules.set_item("rumblelib", attrs.clone().into_object(), vm)?;
        Ok(())
    };
    vm.unwrap_pyresult(run());
    attrs
}

fn py_to_serde<T: serde::de::DeserializeOwned>(
    py: &PyObjectRef,
    vm: &VirtualMachine,
) -> ProgramResult<T> {
    let val = py_serde::serialize(vm, py, serde_json::value::Serializer)?;
    let out = serde_json::from_value(val)?;
    Ok(out)
}

fn invoke_main(main: &PyObjectRef, input: serde_json::Value, vm: &VirtualMachine) -> ProgramResult {
    let input = py_serde::deserialize(vm, input)?;
    let ret = vm.invoke(main, vec![input]).map_err(|e| {
        eprintln!("error in stdlib init:");
        rustpython_vm::exceptions::print_exception(vm, e);
        ProgramError::InternalError
    })?;
    py_to_serde(&ret, vm).and_then(|r| r)
}

fn __init(code: &str) -> ProgramResult<impl FnMut(serde_json::Value) -> ProgramResult> {
    let interp = Interpreter::new(PySettings::default(), InitParameter::Internal);
    let main = interp.enter(|vm| {
        let code = vm
            .compile(
                code,
                rustpython_vm::compile::Mode::Exec,
                "<robot>".to_owned(),
            )
            .map_err(|err| {
                ProgramError::InitError(logic::Error {
                    summary: err.to_string(),
                    details: None,
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
        match make_main() {
            Ok(f) => Ok(f),
            Err(exc) => {
                // if setup errors, try to format the error, and just return an InternalError if it
                // doesn't work
                let exc = vm
                    .invoke(&formatexc, vec![exc.into_object()])
                    .map_err(|_| ProgramError::InternalError)?;
                let err = py_to_serde(&exc, &vm).map_err(|_| ProgramError::InternalError)?;
                Err(ProgramError::InitError(err))
            }
        }
    })?;

    Ok(move |input| interp.enter(|vm| invoke_main(&main, input, vm)))
}

include!("../../lang-common.rs");
