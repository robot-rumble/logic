use std::cmp::Ordering;
use std::collections::HashMap;

use futures_util::future::{join_all, FutureExt};
use itertools::Itertools;
use maplit::hashmap;
use multimap::MultiMap;
use rand::Rng;
use strum::IntoEnumIterator;

pub use types::*;

mod types;

pub fn randrange(low: usize, high: usize) -> usize {
    let mut rng = rand::thread_rng();
    rng.gen_range(low, high)
}

pub fn randchoose<T: Copy>(v: &Vec<T>) -> T {
    match v.len() {
        0 => panic!("Cannot randomly choose from empty list"),
        1 => v[0],
        _ => v[randrange(0, v.len() - 1)],
    }
}

pub fn new_id() -> Id {
    use std::sync::atomic;
    static COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(1);
    Id(COUNTER.fetch_add(1, atomic::Ordering::Relaxed))
}

impl Obj {
    const UNIT_HEALTH: usize = 5;
    const ATTACK_POWER: usize = 1;

    pub fn new_terrain(type_: TerrainType, coords: Coords) -> Self {
        Self(
            Self::new_basic_obj(coords),
            ObjDetails::Terrain(Terrain { type_ }),
        )
    }

    pub fn new_unit(type_: UnitType, coords: Coords, team: Team) -> Self {
        Self(
            Self::new_basic_obj(coords),
            ObjDetails::Unit(Unit {
                type_,
                team,
                health: Self::UNIT_HEALTH,
            }),
        )
    }

    fn new_basic_obj(coords: Coords) -> BasicObj {
        BasicObj {
            coords,
            id: new_id(),
        }
    }

    fn id(&self) -> Id {
        self.0.id
    }
    fn coords(&self) -> Coords {
        self.0.coords
    }
    fn details(&self) -> &ObjDetails {
        &self.1
    }
}

impl State {
    const TEAM_UNIT_NUM: usize = 10;
    const SPAWN_EVERY: usize = 10;

    pub fn new(grid_type: MapType, grid_size: usize) -> Self {
        // create initial objs/map combination
        let (objs, spawn_points) = Self::init(grid_type, grid_size);
        let grid = Self::create_grid_map(&objs);

        Self {
            objs,
            grid,
            spawn_points,
        }
    }

    fn create_raw_grid(size: usize) -> Vec<Coords> {
        (0..size)
            .map(|x| (0..size).map(move |y| Coords(x, y)))
            .flatten()
            .collect()
    }
    fn init(type_: MapType, size: usize) -> (ObjMap, Vec<Coords>) {
        let distance_from_center = |Coords(x, y)| {
            ((size / 2) as i32 - x as i32).pow(2) + ((size / 2) as i32 - y as i32).pow(2)
        };

        let grid = Self::create_raw_grid(size);
        let objs = grid
            .iter()
            .filter(|coords| {
                let coords = *coords;
                match type_ {
                    MapType::Rect => {
                        coords.0 == 0
                            || coords.0 == size - 1
                            || coords.1 == 0
                            || coords.1 == size - 1
                    }
                    MapType::Circle => distance_from_center(*coords) >= (size / 2).pow(2) as i32,
                }
            })
            .map(|coords| {
                let obj = Obj::new_terrain(TerrainType::Wall, *coords);
                (obj.id(), obj)
            })
            .collect();
        let spawn_points = grid
            .into_iter()
            .filter(|coords| match type_ {
                MapType::Rect => {
                    coords.0 == 1 || coords.0 == size - 2 || coords.1 == 1 || coords.1 == size - 2
                }
                MapType::Circle => distance_from_center(*coords) >= (size / 2 - 1).pow(2) as i32,
            })
            .collect();
        (objs, spawn_points)
    }

    fn create_grid_map(objs: &ObjMap) -> GridMap {
        objs.values().map(|obj| (obj.coords(), obj.id())).collect()
    }

    fn spawn_units(&mut self) {
        let objs = (0..Self::TEAM_UNIT_NUM)
            .map(|_| {
                Team::iter()
                    .map(|team| {
                        self.random_spawn_loc().map(|loc| {
                            let obj = Obj::new_unit(UnitType::Soldier, loc, team);
                            // update the grid continuously so random_grid_loc can account for new units
                            self.grid.insert(obj.coords(), obj.id());
                            (obj.id(), obj)
                        })
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();
        self.objs.extend(objs);
    }

    fn random_spawn_loc(&self) -> Option<Coords> {
        let available_points = self
            .spawn_points
            .iter()
            .filter(|loc| !self.grid.contains_key(&loc))
            .collect::<Vec<_>>();
        if available_points.is_empty() {
            None
        } else {
            Some(*randchoose(&available_points))
        }
    }

    fn create_team_map(objs: &ObjMap) -> TeamMap {
        objs.values()
            .filter_map(|obj| match obj.details() {
                ObjDetails::Unit(unit) => Some((unit.team, obj.id())),
                _ => None,
            })
            .collect::<MultiMap<Team, Id>>()
            .into_iter()
            .collect()
    }

    fn determine_winner(self) -> Option<Team> {
        let mut reds = 0;
        let mut blues = 0;
        for (_, obj) in self.objs {
            if let ObjDetails::Unit(unit) = obj.details() {
                match unit.team {
                    Team::Red => reds += 1,
                    Team::Blue => blues += 1,
                }
            }
        }
        match reds.cmp(&blues) {
            Ordering::Less => Some(Team::Blue),
            Ordering::Greater => Some(Team::Red),
            Ordering::Equal => None,
        }
    }
}

impl ProgramInput {
    pub fn new(turn_state: TurnState, team: Team, grid_size: usize) -> Self {
        let TurnState { turn, state } = turn_state;
        let teams = State::create_team_map(&state.objs);
        Self {
            state: StateForProgramInput {
                turn,
                objs: state.objs,
                grid: state.grid,
                teams,
            },
            team,
            grid_size,
        }
    }
}

fn validate_robot_action(
    action: ActionResult,
    team: Team,
    id: Id,
    objs: &ObjMap,
) -> ValidatedRobotAction {
    action
        .map_err(RobotErrorAfterValidation::RuntimeError)
        .and_then(|action| match objs.get(&id).map(|obj| obj.details()) {
            Some(ObjDetails::Unit(unit)) if unit.team != team => {
                Err(RobotErrorAfterValidation::InvalidAction(
                    "Action ID points to unit on other team".into(),
                ))
            }
            Some(ObjDetails::Terrain(_)) => Err(RobotErrorAfterValidation::InvalidAction(
                "Action ID points to terrain".into(),
            )),
            None => Err(RobotErrorAfterValidation::InvalidAction(
                "Action ID points to nonexistent object".into(),
            )),
            _ => Ok(action),
        })
}

fn is_id_valid(team: Team, id: Id, objs: &ObjMap) -> bool {
    match objs.get(&id).map(|obj| obj.details()) {
        Some(ObjDetails::Unit(unit)) => unit.team == team,
        _ => false,
    }
}

fn handle_program_errors<T>(
    errored_players: (
        (Team, Result<T, ProgramError>),
        (Team, Result<T, ProgramError>),
    ),
    turns: Vec<CallbackInput>,
) -> MainOutput {
    let mut errors = HashMap::new();
    let winner = match errored_players {
        ((t1, Err(e1)), (t2, Err(e2))) => {
            errors.insert(t1, e1);
            errors.insert(t2, e2);
            None
        }
        ((t1, Err(e1)), (t2, Ok(_))) => {
            errors.insert(t1, e1);
            Some(t2)
        }
        ((t1, Ok(_)), (t2, Err(e2))) => {
            errors.insert(t2, e2);
            Some(t1)
        }
        _ => unreachable!(),
    };
    MainOutput {
        winner,
        errors,
        turns,
    }
}

const GRID_SIZE: usize = 19;

#[cfg_attr(not(feature = "robot-runner-not-send"), async_trait::async_trait)]
#[cfg_attr(feature = "robot-runner-not-send", async_trait::async_trait(? Send))]
pub trait RobotRunner {
    async fn run(&mut self, input: ProgramInput) -> ProgramResult;
}

#[cfg(not(feature = "robot-runner-not-send"))]
#[async_trait::async_trait]
impl<F> RobotRunner for F
where
    F: FnMut(ProgramInput) -> ProgramResult + Send,
{
    async fn run(&mut self, input: ProgramInput) -> ProgramResult {
        (self)(input)
    }
}

#[cfg(feature = "robot-runner-not-send")]
#[async_trait::async_trait(? Send)]
impl<F> RobotRunner for F
where
    F: FnMut(ProgramInput) -> ProgramResult,
{
    async fn run(&mut self, input: ProgramInput) -> ProgramResult {
        (self)(input)
    }
}

// Team 1: Blue, Team 2: Red
pub async fn run<TurnCb, R>(
    run_team1: Result<R, ProgramError>,
    run_team2: Result<R, ProgramError>,
    mut turn_cb: TurnCb,
    max_turn: usize,
) -> MainOutput
where
    TurnCb: FnMut(&CallbackInput),
    R: RobotRunner,
{
    let mut turns = Vec::new();

    let mut run_funcs = match ((Team::Blue, run_team1), (Team::Red, run_team2)) {
        ((t1, Ok(run_t1)), (t2, Ok(run_t2))) => {
            hashmap! {
                t1 => run_t1,
                t2 => run_t2,
            }
        }
        errored => {
            return handle_program_errors(errored, turns);
        }
    };

    let mut turn_state = TurnState {
        turn: 0,
        state: State::new(MapType::Circle, GRID_SIZE),
    };
    while turn_state.turn < max_turn {
        if turn_state.turn % State::SPAWN_EVERY == 0 {
            turn_state.state.spawn_units();
        }

        turn_state.turn += 1;

        let program_results = join_all(run_funcs.iter_mut().map(|(&team, runner)| {
            runner
                .run(ProgramInput::new(turn_state.clone(), team, GRID_SIZE))
                .map(move |program_result| (team, program_result))
        }))
        .await;

        match program_results.into_iter().collect_tuple().unwrap() {
            ((t1, Ok(output1)), (t2, Ok(output2))) => {
                let team_actions = hashmap! {
                    t1 => output1.robot_actions,
                    t2 => output2.robot_actions,
                };
                let merged_actions = team_actions
                    .into_iter()
                    .flat_map(|(team, output)| output.into_iter().map(move |(k, v)| (k, v, team)))
                    .map(|(id, action, team)| {
                        (
                            id,
                            validate_robot_action(action, team, id, &turn_state.state.objs),
                        )
                    })
                    .collect::<HashMap<Id, ValidatedRobotAction>>();

                run_turn(&merged_actions, &mut turn_state.state);

                let team_logs = hashmap! {
                    t1 => output1.logs,
                    t2 => output2.logs,
                };
                let team_debug_inspections = hashmap! {
                    t1 => output1.debug_inspections,
                    t2 => output2.debug_inspections,
                };
                let team_debug_tables = hashmap! {
                    t1 => output1.debug_tables,
                    t2 => output2.debug_tables,
                };
                let merged_debug_tables = team_debug_tables
                    .into_iter()
                    .filter(|(team, debug_tables)| {
                        debug_tables
                            .keys()
                            .all(|id| is_id_valid(*team, *id, &turn_state.state.objs))
                    })
                    .flat_map(|(_team, v)| v)
                    .collect();

                let turn = CallbackInput {
                    state: turn_state.clone(),
                    robot_actions: merged_actions,
                    logs: team_logs,
                    debug_inspections: team_debug_inspections,
                    debug_tables: merged_debug_tables,
                };
                turn_cb(&turn);
                turns.push(turn);
            }
            errored => {
                return handle_program_errors(errored, turns);
            }
        }
    }
    let winner = turn_state.state.determine_winner();
    MainOutput {
        winner,
        errors: HashMap::new(),
        turns,
    }
}

fn run_turn(robot_actions: &HashMap<Id, ValidatedRobotAction>, state: &mut State) {
    let mut movement_map = MultiMap::new();
    let mut attack_map = MultiMap::new();

    for (id, action) in robot_actions
        .iter()
        .filter_map(|(id, action)| action.as_ref().ok().map(|a| (id, a)))
    {
        let map = match action.type_ {
            ActionType::Move => &mut movement_map,
            ActionType::Attack => &mut attack_map,
        };
        let obj = state.objs.get(&id).unwrap();
        map.insert(obj.coords() + action.direction, id);
    }

    let movement_grid = movement_map
        .iter()
        .filter_map(|(coords, &id)| {
            if movement_map.is_vec(coords) {
                None
            } else {
                Some((*coords, *id))
            }
        })
        .collect::<GridMap>();

    state
        .grid
        .retain(|_, id| !movement_grid.values().any(|movement_id| id == movement_id));
    update_grid_with_movement(&mut state.objs, &mut state.grid, movement_grid);

    for (coords, attacks) in attack_map.iter_all() {
        let attack_power = attacks.len() * Obj::ATTACK_POWER;
        match state.grid.get(coords) {
            Some(id) => {
                if let Some(ObjDetails::Unit(ref mut unit)) =
                    state.objs.get_mut(id).map(|obj| &mut obj.1)
                {
                    unit.health = unit.health.saturating_sub(attack_power);
                    if unit.health == 0 {
                        state.objs.remove(id).unwrap();
                        state.grid.remove(coords).unwrap();
                    }
                }
            }
            None => (),
        };
    }
}

pub fn update_grid_with_movement(objs: &mut ObjMap, grid: &mut GridMap, movement_grid: GridMap) {
    let (illegal_moves, legal_moves): (GridMap, GridMap) = movement_grid
        .into_iter()
        .partition(|(coords, _)| grid.contains_key(coords));

    if illegal_moves.is_empty() {
        for (&coords, id) in legal_moves.iter() {
            objs.get_mut(id).unwrap().0.coords = coords
        }
        grid.extend(legal_moves)
    } else {
        // insert the units with illegal moves back in their original location
        for (_, id) in illegal_moves.into_iter() {
            grid.insert(objs.get(&id).unwrap().0.coords, id);
        }
        update_grid_with_movement(objs, grid, legal_moves);
    }
}
