use std::collections::HashMap;
use std::ops::{Add, Mul};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum MapType {
    Rect
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Team {
    Red,
    Blue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MainOutput {
    pub winner: Team,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct Id(pub usize);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TurnState {
    pub turn: usize,
    pub state: State,
}

pub type ObjMap = HashMap<Id, Obj>;
pub type GridMap = HashMap<Coords, Id>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub objs: ObjMap,
    pub grid: GridMap,
}

pub type TeamMap = HashMap<Team, Vec<Id>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StateForRobotInput {
    pub objs: ObjMap,
    pub grid: GridMap,
    pub teams: TeamMap,
    pub turn: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotInput {
    pub state: StateForRobotInput,
    pub grid_size: usize,
    pub team: Team,
}

pub type ActionMap = HashMap<Id, Action>;

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotOutput {
    pub actions: ActionMap,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct Coords(pub usize, pub usize);

impl Add for Coords {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Add<Direction> for Coords {
    type Output = Self;

    fn add(self, rhs: Direction) -> Self {
        let (dir_x, dir_y) = rhs.to_tuple();
        Self(
            if dir_x < 0 { self.0.saturating_sub(dir_x as usize) } else { self.0 + dir_x as usize },
            if dir_y < 0 { self.1.saturating_sub(dir_y as usize) } else { self.1 + dir_y as usize },
        )
    }
}

impl Mul<usize> for Coords {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self {
        Self(self.0 * rhs, self.1 * rhs)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Obj(pub BasicObj, pub ObjDetails);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicObj {
    pub id: Id,
    pub coords: Coords,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ObjDetails {
    Terrain(Terrain),
    Unit(Unit),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Terrain {
    pub type_: TerrainType,
}


#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum TerrainType {
    Wall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Unit {
    pub type_: UnitType,
    pub team: Team,
    pub health: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum UnitType {
    Soldier,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub type_: ActionType,
    pub direction: Direction,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum ActionType {
    Move,
    Attack,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    fn to_tuple(self) -> (isize, isize) {
        use Direction::*;
        match self {
            West => (-1, 0),
            North => (0, -1),
            East => (1, 0),
            South => (0, 1),
        }
    }
}
