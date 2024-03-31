use rustpython_vm::object::PyObjectRef;
use rustpython_vm::py_serde;
use rustpython_vm::Interpreter;
use rustpython_vm::VirtualMachine;

use logic::{ProgramError, ProgramResult};

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
    let ret = main.call(vec![input], vm).map_err(|e| {
        eprintln!("error in stdlib init:");
        vm.print_exception(e);
        ProgramError::InternalError
    })?;
    py_to_serde(&ret, vm).and_then(|r| r)
}

fn __init(code: &str) -> ProgramResult<impl FnMut(serde_json::Value) -> ProgramResult> {
    let interp = Interpreter::with_init(Default::default(), |vm| {
        vm.add_native_modules(rustpython_stdlib::get_module_inits());
        vm.add_frozen(rustpython_pylib::FROZEN_STDLIB);
    });

    let main = interp.enter(|vm| {
        let code = vm
            .compile(
                &*code,
                rustpython_vm::compiler::Mode::Exec,
                "<robot>".to_owned(),
            )
            .map_err(|err| {
                ProgramError::InitError(logic::Error {
                    summary: err.to_string(),
                    details: None,
                    loc: err.location.map(|loc| logic::ErrorLoc {
                        start: (loc.row.to_usize(), Some(loc.column.to_usize())),
                        end: None,
                    }),
                })
            })?;

        let scope = vm.new_scope_with_builtins();
        let make_main = || {
            let rumblelib_code = vm
                .compile(
                    include_str!("../stdlib/rumblelib.py"),
                    rustpython_vm::compiler::Mode::Exec,
                    "<robot>".to_owned(),
                ).unwrap();
            vm.run_code_obj(rumblelib_code, scope.clone())?;
            let rumblelib = vm.new_module("rumblelib", scope.globals.clone(), None);
            rumblelib.dict().set_item("__name__", vm.ctx.new_str("<robot>".to_owned()).into(), vm)?;
            let sys_modules = vm.sys_module.get_attr("modules", vm)?;
            sys_modules.set_item("rumblelib", rumblelib.clone().into(), vm)?;

            vm.run_code_obj(code, scope.clone())?;
            rumblelib.dict().get_item("__main", &vm).map_err(|_| {
                vm.new_type_error(
                    "you must **not** delete the `__main` function, c'mon, dude".to_owned(),
                )
            })
        };
        match make_main() {
            Ok(f) => Ok(f),
            Err(exc) => {
                let formatexc = vm.unwrap_pyresult(scope.globals.get_item("__format_err", &vm));
                // if setup errors, try to format the error, and just return an InternalError if it
                // doesn't work
                let exc = formatexc
                    .call(vec![exc.into()], vm)
                    .map_err(|_| ProgramError::InternalError)?;
                let err = py_to_serde(&exc, &vm).map_err(|_| ProgramError::InternalError)?;
                Err(ProgramError::InitError(err))
            }
        }
    })?;

    Ok(move |input| interp.enter(|vm| invoke_main(&main, input, vm)))
}

include!("../../lang-common.rs");
