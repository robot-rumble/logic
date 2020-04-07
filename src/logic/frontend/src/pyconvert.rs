// taken from rustpython-wasm

use js_sys::{Error, Reflect, SyntaxError};
use wasm_bindgen::prelude::*;

use rustpython_compiler::error::{CompileError, CompileErrorType};
use rustpython_parser::error::ParseErrorType;
use rustpython_vm::exceptions;
use rustpython_vm::exceptions::PyBaseExceptionRef;
use rustpython_vm::pyobject::PyResult;
use rustpython_vm::VirtualMachine;

#[wasm_bindgen(inline_js = r"
export class PyError extends Error {
    constructor(info) {
        const msg = info.args[0];
        if (typeof msg === 'string') super(msg);
        else super();
        this.info = info;
    }
    get name() { return this.info.exc_type; }
    get traceback() { return this.info.traceback; }
    toString() { return this.info.rendered; }
}
")]
extern "C" {
    pub type PyError;
    #[wasm_bindgen(constructor)]
    fn new(info: JsValue) -> PyError;
}

pub fn py_err_to_js_err(vm: &VirtualMachine, py_err: &PyBaseExceptionRef) -> JsValue {
    let res = JsValue::from_serde(&exceptions::SerializeException::new(vm, py_err));
    match res {
        Ok(err_info) => PyError::new(err_info).into(),
        Err(e) => Error::new(&e.to_string()).into(),
    }
}

pub fn syntax_err(err: CompileError) -> SyntaxError {
    let js_err = SyntaxError::new(&format!("Error parsing Python code: {}", err));
    let _ = Reflect::set(&js_err, &"row".into(), &(err.location.row() as u32).into());
    let _ = Reflect::set(
        &js_err,
        &"col".into(),
        &(err.location.column() as u32).into(),
    );
    let can_continue = match &err.error {
        CompileErrorType::Parse(ParseErrorType::EOF) => true,
        _ => false,
    };
    let _ = Reflect::set(&js_err, &"canContinue".into(), &can_continue.into());
    js_err
}

pub trait PyResultExt<T> {
    fn to_js(self, vm: &VirtualMachine) -> Result<T, JsValue>;
}
impl<T> PyResultExt<T> for PyResult<T> {
    fn to_js(self, vm: &VirtualMachine) -> Result<T, JsValue> {
        self.map_err(|err| py_err_to_js_err(vm, &err))
    }
}
