use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use logic::{StateForRobotInput as State, Team};

use rustpython_vm as vm;
use vm::function::PyFuncArgs;
use vm::obj::objstr::PyStringRef;
use vm::obj::objtype::PyClassRef;
use vm::pyobject::{ItemProtocol, PyClassImpl, PyContext, PyObjectRef, PyRef, PyResult, PyValue};
use vm::VirtualMachine;
use vm::{extend_module, py_compile_bytecode, pyclass, pyimpl};

fn parse_enum<T: std::str::FromStr>(
    name: &str,
    example: &str,
    s: &str,
    vm: &VirtualMachine,
) -> PyResult<T> {
    s.parse().map_err(|_| {
        vm.new_type_error(format!(
            "invalid {} '{}'. make sure it's capitalized correctly, e.g. '{}'",
            name, s, example,
        ))
    })
}

#[pyclass]
#[derive(Debug)]
struct Coords(logic::Coords);
impl PyValue for Coords {
    fn class(vm: &VirtualMachine) -> PyClassRef {
        vm.class("builtins", "Coords")
    }
}
#[pyimpl]
impl Coords {
    #[pyslot]
    fn tp_new(cls: PyClassRef, x: usize, y: usize, vm: &VirtualMachine) -> PyResult<PyRef<Self>> {
        Self(logic::Coords(x, y)).into_ref_with_type(vm, cls)
    }
    fn coords(&self) -> logic::Coords {
        self.0
    }
    #[pyproperty]
    fn x(&self) -> usize {
        self.coords().0
    }
    #[pyproperty]
    fn y(&self) -> usize {
        self.coords().1
    }
}

#[pyclass]
#[derive(Debug)]
pub struct Obj(pub logic::Obj);
impl PyValue for Obj {
    fn class(vm: &VirtualMachine) -> PyClassRef {
        vm.class("builtins", "Obj")
    }
}
#[pyimpl]
impl Obj {
    #[pyslot]
    fn tp_new(_args: PyFuncArgs, vm: &VirtualMachine) -> PyResult<PyRef<Self>> {
        Err(vm.new_type_error("cannot create 'Obj' instances".to_owned()))
    }

    fn terrain(&self, op: &str, vm: &VirtualMachine) -> PyResult<&logic::Terrain> {
        match (self.0).1 {
            logic::ObjDetails::Terrain(ref t) => Ok(t),
            _ => Err(vm.new_type_error(format!("object is not a Terrain, cannot {}", op))),
        }
    }
    fn unit(&self, op: &str, vm: &VirtualMachine) -> PyResult<&logic::Unit> {
        match (self.0).1 {
            logic::ObjDetails::Unit(ref u) => Ok(u),
            _ => Err(vm.new_type_error(format!("object is not a Unit, cannot {}", op))),
        }
    }

    #[pyproperty]
    fn r#type(&self) -> &'static str {
        <&str>::from(&(self.0).1)
    }
    #[pyproperty]
    fn terrain_type(&self, vm: &VirtualMachine) -> PyResult<&'static str> {
        self.terrain("get terrain type", vm).map(|t| t.type_.into())
    }
    #[pyproperty]
    fn unit_type(&self, vm: &VirtualMachine) -> PyResult<&'static str> {
        self.unit("get unit type", vm).map(|u| u.type_.into())
    }
    #[pyproperty]
    fn unit_team(&self, vm: &VirtualMachine) -> PyResult<&'static str> {
        self.unit("get unit team", vm).map(|u| u.team.into())
    }
    #[pyproperty]
    fn unit_health(&self, vm: &VirtualMachine) -> PyResult<usize> {
        self.unit("get unit health", vm).map(|u| u.health)
    }
}

#[pyclass]
#[derive(Debug)]
pub struct Action(pub logic::Action);
impl PyValue for Action {
    fn class(vm: &VirtualMachine) -> PyClassRef {
        vm.class("builtins", "Action")
    }
}
#[pyimpl]
impl Action {
    #[pyslot]
    fn tp_new(
        cls: PyClassRef,
        ty: PyStringRef,
        dir: PyStringRef,
        vm: &VirtualMachine,
    ) -> PyResult<PyRef<Self>> {
        let type_ = parse_enum("action type", "Move", ty.as_str(), vm)?;
        let direction = parse_enum("direction", "North", dir.as_str(), vm)?;
        Self(logic::Action { type_, direction }).into_ref_with_type(vm, cls)
    }
    #[pymethod(magic)]
    fn repr(&self) -> String {
        format!("<Action: {:?} {:?}>", self.0.type_, self.0.direction)
    }
}

fn make_action_func(type_: logic::ActionType, ctx: &PyContext) -> PyObjectRef {
    ctx.new_function(move |dir: PyStringRef, vm: &VirtualMachine| {
        let direction = parse_enum("direction", "North", dir.as_str(), vm)?;
        Ok(Action(logic::Action { type_, direction }))
    })
}

pub fn add(
    state_ref: &Rc<RefCell<State>>,
    cur_team_ref: &Rc<Cell<Team>>,
    log: &js_sys::Function,
    vm: &VirtualMachine,
) {
    let ctx = &vm.ctx;

    let state = state_ref.clone();
    let obj_by_id = move |id: usize, vm: &VirtualMachine| -> PyResult<Obj> {
        state
            .borrow()
            .objs
            .get(&logic::Id(id))
            .map(|obj| Obj(obj.clone()))
            .ok_or_else(|| vm.new_lookup_error("no object with that id".to_owned()))
    };

    let state = state_ref.clone();
    let objs_by_team = move |team: PyStringRef, vm: &VirtualMachine| -> PyResult {
        let team = parse_enum("team", "Red", team.as_str(), vm)?;
        let state = state.borrow();
        let objs = state.teams[&team]
            .iter()
            .map(|id| {
                Obj(state.objs.get(id).unwrap().clone())
                    .into_ref(vm)
                    .into_object()
            })
            .collect();
        Ok(vm.ctx.new_list(objs))
    };

    let state = state_ref.clone();
    let ids_by_team = move |team: PyStringRef, vm: &VirtualMachine| -> PyResult {
        let team = parse_enum("team", "Red", team.as_str(), vm)?;
        let state = state.borrow();
        let objs = state.teams[&team]
            .iter()
            .map(|id| vm.new_int(id.0))
            .collect();
        Ok(vm.ctx.new_list(objs))
    };

    let state = state_ref.clone();
    let obj_by_loc = move |coord: PyRef<Coords>| -> Option<Obj> {
        let state = state.borrow();
        state
            .grid
            .get(&coord.coords())
            .map(|id| Obj(state.objs.get(id).unwrap().clone()))
    };

    let state = state_ref.clone();
    let id_by_loc = move |coord: PyRef<Coords>| -> Option<usize> {
        state.borrow().grid.get(&coord.coords()).map(|id| id.0)
    };

    let red = vm.new_str("Red".to_owned());
    let blue = vm.new_str("Blue".to_owned());
    let cur_team = cur_team_ref.clone();
    let other_team = move || -> PyObjectRef {
        match cur_team.get() {
            Team::Red => blue.clone(),
            Team::Blue => red.clone(),
        }
    };

    extend_module!(vm, &vm.builtins, {
        "Coords" => Coords::make_class(ctx),
        "Obj" => Obj::make_class(ctx),
        "Action" => Action::make_class(ctx),
        "move" => make_action_func(logic::ActionType::Move, ctx),
        "attack" => make_action_func(logic::ActionType::Attack, ctx),
        "obj_by_id" => ctx.new_function(obj_by_id),
        "objs_by_team" => ctx.new_function(objs_by_team),
        "ids_by_team" => ctx.new_function(ids_by_team),
        "obj_by_loc" => ctx.new_function(obj_by_loc),
        "id_by_loc" => ctx.new_function(id_by_loc),
        "other_team" => ctx.new_function(other_team),
    });

    let stdio = {
        let (_, frozen): (String, _) = py_compile_bytecode!(
            source = "
import io
class RobotRumbleLoggingIO(io.TextIOBase):
    def write(self, s):
        _log(s)
        return len(s)
    def flush(self):
        pass
",
            module_name = "__debugoutput"
        )
        .into_iter()
        .next()
        .unwrap();

        let attrs = vm.ctx.new_dict();
        let log = log.clone();
        attrs
            .set_item(
                "_log",
                ctx.new_function(move |s: PyStringRef| {
                    log.call1(&wasm_bindgen::JsValue::UNDEFINED, &s.as_str().into())
                        .expect("log callback failed");
                }),
                vm,
            )
            .unwrap();
        vm.run_code_obj(
            vm.ctx.new_code_object(frozen.code),
            vm::scope::Scope::with_builtins(None, attrs.clone(), vm),
        )
        .expect("debugoutput should not fail");
        let cls = attrs.get_item("RobotRumbleLoggingIO", vm).unwrap();
        vm.invoke(&cls, vec![]).unwrap()
    };

    extend_module!(vm, &vm.sys_module, {
        "__stdout__" => stdio.clone(),
        "stdout" => stdio.clone(),
        "__stderr__" => stdio.clone(),
        "stderr" => stdio,
    });
}
