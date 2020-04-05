use std::collections::HashMap;

use types::*;

#[allow(dead_code)]
pub mod types;

pub fn run<RunF, TurnCb, FinishCb>(
    run_team: RunF,
    turn_cb: TurnCb,
    finish_cb: FinishCb,
    max_turn: usize,
) where
    RunF: Fn(Team, RobotInput) -> RobotOutput,
    TurnCb: Fn(TurnState) -> (),
    FinishCb: Fn(MainOutput) -> (),
{
    let mut objs = HashMap::new();
    objs.insert(Id("asdf".into()), Obj(BasicObj { id: Id("asdf".into()), coords: Coords(0, 0) }, ObjDetails::Unit(Unit { type_: UnitType::Soldier, health: 5, team: Team::Red })));
    turn_cb(TurnState { turn: 0, objs });
    finish_cb(MainOutput { winner: Team::Red });
}
