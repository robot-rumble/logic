use rustpython_vm::obj::objcode::PyCodeRef;
use rustpython_vm::obj::objfunction::PyFunctionRef;
use rustpython_vm::pyobject::{ItemProtocol, PyResult, PyValue};
use rustpython_vm::scope::Scope;
use rustpython_vm::VirtualMachine;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

mod stdlib;

pub fn run_python(
    code1: PyCodeRef,
    code2: PyCodeRef,
    turn_callback: impl FnMut(&logic::TurnState),
    log_callback: impl Fn(&str) + Clone + 'static,
    turn_num: usize,
    vm: &VirtualMachine,
) -> PyResult<logic::MainOutput> {
    let py_state: Rc<RefCell<logic::StateForRobotInput>> = Rc::default();
    let py_cur_team = Rc::new(Cell::new(logic::Team::Red));

    let create_robot_fn = |code: PyCodeRef| -> PyResult<PyFunctionRef> {
        let attrs = vm.ctx.new_dict();
        attrs.set_item("__name__", vm.new_str("<robot>".to_owned()), vm)?;

        // Execute main code in module:
        vm.run_code_obj(code.clone(), Scope::with_builtins(None, attrs.clone(), vm))?;

        stdlib::add(&py_state, &py_cur_team, log_callback.clone(), vm);

        let robot = attrs
            .get_item_option("robot", vm)?
            .ok_or_else(|| vm.new_type_error("you must define a 'robot' function".to_owned()))?;

        let robot: PyFunctionRef = robot
            .downcast()
            .map_err(|_| vm.new_type_error("'robot' should be a function".to_owned()))?;

        // TODO(noah): add a .code() getter to PyFunction
        let code: PyCodeRef = vm
            .get_attribute(robot.as_object().clone(), "__code__")
            .unwrap()
            .downcast()
            .unwrap();
        if code.arg_names.len() != 2 {
            let msg =
                "Your 'robot' function must accept two values: the current turn and robot details.";
            return Err(vm.new_type_error(msg.to_owned()));
        }

        Ok(robot)
    };

    let red = create_robot_fn(code1)?;
    let blue = create_robot_fn(code2)?;

    let run_team = |team, input: logic::RobotInput| -> PyResult<_> {
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
            .map(|id| -> PyResult<_> {
                let obj = stdlib::Obj(state.objs[id].clone())
                    .into_ref(vm)
                    .into_object();
                let ret = robot.invoke(vec![turn.clone(), obj].into(), vm)?;
                let action = ret.payload::<stdlib::Action>().ok_or_else(|| {
                    vm.new_type_error("Robot did not return an action!".to_owned())
                })?;
                Ok((*id, action.0))
            })
            .collect::<Result<_, _>>()?;

        Ok(logic::RobotOutput { actions })
    };

    logic::run(run_team, turn_callback, turn_num)
}
