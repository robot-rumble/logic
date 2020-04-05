use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum Team {
    Red,
    Blue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MainOutput {
    pub winner: Team,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Id(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct TurnState {
    pub turn: usize,
    pub objs: HashMap<Id, Obj>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdditionalState {
    pub teams: HashMap<Team, Vec<Id>>,
    pub map: Vec<Vec<Id>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotInputState {
    pub basic: TurnState,
    pub additional: AdditionalState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotInput {
    pub state: RobotInputState,
    pub team: Team,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotOutput {
    pub actions: HashMap<Id, Action>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Coords(pub usize, pub usize);

#[derive(Serialize, Deserialize, Debug)]
pub struct Obj(pub BasicObj, pub ObjDetails);

#[derive(Serialize, Deserialize, Debug)]
pub struct BasicObj {
    pub id: Id,
    pub coords: Coords,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ObjDetails {
    Terrain(Terrain),
    Unit(Unit),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Terrain {
    pub type_: TerrainType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TerrainType {
    Wall,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Unit {
    pub type_: UnitType,
    pub team: Team,
    pub health: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UnitType {
    Soldier,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub type_: ActionType,
    pub direction: Direction,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ActionType {
    Move,
    Attack,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
