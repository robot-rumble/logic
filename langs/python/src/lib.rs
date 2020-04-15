use rustpython_vm::obj::objdict::PyDictRef;
use rustpython_vm::obj::objstr::PyStringRef;
use rustpython_vm::py_compile_bytecode;
use rustpython_vm::pyobject::{ItemProtocol, PyObjectRef, TryFromObject};
use rustpython_vm::scope::Scope;
use rustpython_vm::{InitParameter, PySettings, VirtualMachine};

use logic::{ProgramError, ProgramInput, ProgramOutput};
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

fn invoke_main(main: &PyObjectRef, input: &ProgramInput, vm: &VirtualMachine) -> ProgramOutput {
    let run = || {
        let input = vm.new_str(serde_json::to_string(input).unwrap());
        let args = vec![input];
        let ret = vm
            .invoke(main, args)
            .map_err(|_| ProgramError::InternalError)?;
        let json =
            PyStringRef::try_from_object(vm, ret).map_err(|_| ProgramError::InternalError)?;
        Ok(serde_json::from_str(json.as_str())?)
    };
    run().unwrap_or_else(|err| ProgramOutput {
        robot_outputs: Err(err),
        logs: Vec::new(),
    })
}

pub fn init(code: &str) -> Result<impl FnMut(ProgramInput) -> ProgramOutput, ProgramError> {
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
            ProgramError::InitError(logic::RobotError {
                start: (err.location.row(), Some(err.location.column())),
                end: None,
                message: err.to_string(),
            })
        })?;

    let attrs = setup_scope(&vm);
    let formatexc = attrs.get_item("__format_exc", &vm).unwrap();

    let make_main = || {
        vm.run_code_obj(code, Scope::with_builtins(None, attrs.clone(), &vm))?;
        attrs.get_item("_main", &vm).map_err(|_| {
            vm.new_type_error(
                "you must **not** delete the `_main` function, c'mon, dude".to_owned(),
            )
        })
    };
    let main = match make_main() {
        Ok(f) => f,
        Err(exc) => {
            // if setup errors, try to format the error, and just return an InternalError if it
            // doesn't work
            let exc = vm
                .invoke(&formatexc, vec![exc.into_object(), vm.new_bool(true)])
                .map_err(|_| ProgramError::InternalError)?;
            let json =
                PyStringRef::try_from_object(&vm, exc).map_err(|_| ProgramError::InternalError)?;
            let err =
                serde_json::from_str(json.as_str()).map_err(|_| ProgramError::InternalError)?;
            return Err(ProgramError::InitError(err));
        }
    };

    Ok(move |input| invoke_main(&main, &input, &vm))
}
