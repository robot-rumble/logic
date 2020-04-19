use core::fmt;
use std::collections::HashMap;
use std::ops::Add;

use serde::{Deserialize, Serialize};
use strum::*;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum MapType {
    Rect,
}

#[derive(
    Serialize,
    Deserialize,
    EnumIter,
    EnumString,
    IntoStaticStr,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Copy,
    Clone,
)]
pub enum Team {
    Red,
    Blue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MainOutput {
    pub winner: Option<Team>,
    pub errors: HashMap<Team, ProgramError>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Copy, Clone)]
#[serde(transparent)]
pub struct Id(#[serde(with = "serde_with::rust::display_fromstr")] pub usize);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TurnState {
    pub turn: usize,
    #[serde(flatten)]
    pub state: State,
}

#[derive(Serialize, Deserialize, Error, Clone, Debug)]
pub enum RobotErrorAfterValidation {
    #[error("Robot function error")]
    RuntimeError(Error),
    #[error("Invald action")]
    InvalidAction(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidatedRobotOutput {
    pub action: Result<Action, RobotErrorAfterValidation>,
    pub debug_table: DebugTable,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CallbackInput {
    pub state: TurnState,
    // logs are on the level of the team
    pub logs: HashMap<Team, Logs>,
    // debug_tables are on the level of the individual robots
    pub robot_outputs: HashMap<Id, ValidatedRobotOutput>,
}

pub type ObjMap = HashMap<Id, Obj>;

type GridMapType = HashMap<Coords, Id>;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(from = "SerdeGridMap", into = "SerdeGridMap")]
pub struct GridMap(GridMapType);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub objs: ObjMap,
    pub grid: GridMap,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in SerdeGridMap::from(self.grid.clone()).0 {
            for col in row {
                let s = match col {
                    Some(id) => {
                        let obj = self.objs.get(&id).unwrap();
                        match &obj.1 {
                            ObjDetails::Terrain(_) => "■",
                            ObjDetails::Unit(unit) => match unit.team {
                                Team::Red => "\x1b[41;1mr\x1b[0m",
                                Team::Blue => "\x1b[44;1mb\x1b[0m",
                            },
                        }
                    }
                    None => " ",
                };
                write!(f, " {}", s)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

pub type TeamMap = HashMap<Team, Vec<Id>>;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct StateForProgramInput {
    pub objs: ObjMap,
    pub grid: GridMap,
    pub teams: TeamMap,
    pub turn: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProgramInput {
    #[serde(flatten)]
    pub state: StateForProgramInput,
    pub grid_size: usize,
    pub team: Team,
}

pub type Range = (usize, Option<usize>);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorLoc {
    pub start: Range,
    pub end: Option<Range>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Error {
    pub message: String,
    pub loc: Option<ErrorLoc>,
}

pub type DebugTable = HashMap<String, String>;

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotOutput {
    pub action: Result<Action, Error>,
    pub debug_table: DebugTable,
}

pub type RobotOutputMap = HashMap<Id, RobotOutput>;

#[derive(Serialize, Deserialize, Error, Debug)]
pub enum ProgramError {
    #[error("Unhandled program error")]
    InternalError,
    #[error("The program exited before it returned any data")]
    NoData,
    #[error("The program errored while initializing")]
    InitError(Error),
    #[error("The program did not output an init status")]
    NoInitError,
    #[error("Program returned invalid data")]
    DataError(String),
    #[error("IO error")]
    IO(String),
}
impl From<serde_json::Error> for ProgramError {
    fn from(err: serde_json::Error) -> Self {
        Self::DataError(err.to_string())
    }
}
impl From<std::io::Error> for ProgramError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err.to_string())
    }
}

pub type ProgramResult<T = RobotOutputMap> = Result<T, ProgramError>;
pub type Logs = Vec<String>;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProgramOutput {
    pub robot_outputs: ProgramResult,
    #[serde(default)]
    pub logs: Vec<String>,
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
            if dir_x < 0 {
                self.0.saturating_sub(dir_x.abs() as usize)
            } else {
                self.0 + dir_x as usize
            },
            if dir_y < 0 {
                self.1.saturating_sub(dir_y.abs() as usize)
            } else {
                self.1 + dir_y as usize
            },
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(from = "SerdeObj", into = "SerdeObj")]
pub struct Obj(pub BasicObj, pub ObjDetails);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicObj {
    pub id: Id,
    pub coords: Coords,
}

#[derive(Serialize, Deserialize, IntoStaticStr, Debug, Clone)]
#[serde(untagged)]
pub enum ObjDetails {
    Terrain(Terrain),
    Unit(Unit),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Terrain {
    #[serde(rename = "type")]
    pub type_: TerrainType,
}

#[derive(Serialize, Deserialize, IntoStaticStr, Debug, PartialEq, Copy, Clone)]
pub enum TerrainType {
    Wall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Unit {
    #[serde(rename = "type")]
    pub type_: UnitType,
    pub team: Team,
    pub health: usize,
}

#[derive(Serialize, Deserialize, IntoStaticStr, Debug, PartialEq, Copy, Clone)]
pub enum UnitType {
    Soldier,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Action {
    #[serde(rename = "type")]
    pub type_: ActionType,
    pub direction: Direction,
}

#[derive(Serialize, Deserialize, EnumString, Debug, PartialEq, Copy, Clone)]
pub enum ActionType {
    Move,
    Attack,
}

#[derive(Serialize, Deserialize, EnumString, Debug, PartialEq, Copy, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SerdeObj {
    #[serde(flatten)]
    basic: BasicObj,
    #[serde(flatten)]
    details: ObjDetails,
}

impl From<Obj> for SerdeObj {
    fn from(Obj(basic, details): Obj) -> Self {
        Self { basic, details }
    }
}
impl From<SerdeObj> for Obj {
    fn from(SerdeObj { basic, details }: SerdeObj) -> Self {
        Obj(basic, details)
    }
}

type SerdeGridMapType = Vec<Vec<Option<Id>>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SerdeGridMap(SerdeGridMapType);

impl From<GridMap> for SerdeGridMap {
    fn from(map: GridMap) -> Self {
        let arr2d = (0..crate::GRID_SIZE)
            .map(|i| {
                (0..crate::GRID_SIZE)
                    .map(|j| map.0.get(&Coords(j, i)).copied())
                    .collect()
            })
            .collect();
        Self(arr2d)
    }
}

impl From<SerdeGridMap> for GridMap {
    fn from(map: SerdeGridMap) -> Self {
        let map = map
            .0
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                v.into_iter()
                    .enumerate()
                    .filter_map(move |(j, elem)| elem.map(|elem| (Coords(i, j), elem)))
            })
            .flatten()
            .collect();
        Self(map)
    }
}

impl std::ops::Deref for GridMap {
    type Target = GridMapType;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for GridMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::iter::FromIterator<(Coords, Id)> for GridMap {
    fn from_iter<T: IntoIterator<Item = (Coords, Id)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl Extend<(Coords, Id)> for GridMap {
    fn extend<T: IntoIterator<Item = (Coords, Id)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}
impl IntoIterator for GridMap {
    type Item = (Coords, Id);
    type IntoIter = <GridMapType as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
