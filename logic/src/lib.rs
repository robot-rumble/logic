use std::cmp::Ordering;
use std::collections::HashMap;

use multimap::MultiMap;
use rand::Rng;
use strum::IntoEnumIterator;

use thiserror::Error;

pub use types::*;

mod types;

pub fn randrange(low: usize, high: usize) -> usize {
    let mut rng = rand::thread_rng();
    rng.gen_range(low, high)
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

    pub fn new(grid_type: MapType, grid_size: usize) -> Self {
        // create initial objs/map combination
        let terrain_objs = Self::create_obj_map(grid_type, grid_size);
        let mut grid = Self::create_grid_map(&terrain_objs);

        // use the map to create the units
        let mut objs: ObjMap = Team::iter()
            .map(|team| Self::create_unit_objs(&mut grid, grid_size, team))
            .flatten()
            .collect();
        objs.extend(terrain_objs);

        Self { objs, grid }
    }

    fn create_raw_grid(size: usize) -> Vec<Coords> {
        (0..size)
            .map(|x| (0..size).map(move |y| Coords(x, y)))
            .flatten()
            .collect()
    }

    fn create_obj_map(type_: MapType, size: usize) -> ObjMap {
        Self::create_raw_grid(size)
            .iter()
            .filter(|Coords(x, y)| match type_ {
                MapType::Rect => *x == 0 || *x == size - 1 || *y == 0 || *y == size - 1,
            })
            .map(|coords| {
                let obj = Obj::new_terrain(TerrainType::Wall, *coords);
                (obj.id(), obj)
            })
            .collect()
    }

    fn create_grid_map(objs: &ObjMap) -> GridMap {
        objs.values().map(|obj| (obj.coords(), obj.id())).collect()
    }

    fn create_unit_objs(grid: &mut GridMap, grid_size: usize, team: Team) -> ObjMap {
        (0..Self::TEAM_UNIT_NUM)
            .map(|_| {
                let obj = Obj::new_unit(
                    UnitType::Soldier,
                    Self::random_grid_loc(grid, grid_size),
                    team,
                );
                // update the grid continuously so random_grid_loc can account for new units
                grid.insert(obj.coords(), obj.id());
                (obj.id(), obj)
            })
            .collect()
    }

    fn random_grid_loc(grid: &GridMap, grid_size: usize) -> Coords {
        let random_coords = Coords(randrange(0, grid_size), randrange(0, grid_size));
        if grid.contains_key(&random_coords) {
            Self::random_grid_loc(grid, grid_size)
        } else {
            random_coords
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

fn validate_robot_output(map: &mut RobotOutputMap, team: Team, objs: &ObjMap) {
    for (id, output) in map.iter_mut() {
        output.action = output.action.and_then(|action| {
            match objs.get(id).map(|obj| obj.details()) {
                Some(ObjDetails::Unit(unit)) if unit.team != team => {
                    ActionError("Action ID points to unit on other team".into())
                }
                Some(ObjDetails::Terrain(_)) => ActionError("Action ID points to terrain".into()),
                None => ActionError("Action ID points to nonexistent object".into()),
                _ => Ok(action),
            }
        });
    }
}

const GRID_SIZE: usize = 19;

pub fn run<RunF, TurnCb>(
    mut run_team_f: HashMap<Team, RunF>,
    mut turn_cb: TurnCb,
    max_turn: usize,
) -> MainOutput
    where
        RunF: FnMut(ProgramInput) -> ProgramOutput,
        TurnCb: FnMut(&CallbackInput) -> (),
{
    let state = State::new(MapType::Rect, GRID_SIZE);
    let mut turn_state = TurnState { turn: 0, state };
    while turn_state.turn < max_turn {
        let (robot_outputs, logs) = Team::iter().map(|team| {
            let program_output = run_team_f[team](ProgramInput::new(turn_state.clone(), team, GRID_SIZE));
            ((team, program_output.robot_outputs), (team, program_output.logs))
        }).unzip();

        let logs = logs.collect::<HashMap<Team, Logs>>();

        turn_state.turn += 1;

        match (robot_outputs[0], robot_outputs[1]) {
            ((t1, Ok(output_map1)), (t2, Ok(output_map2))) => {
                let mut team_outputs = HashMap::new();
                team_outputs.insert(t1, output_map1);
                team_outputs.insert(t2, output_map2);

                for (team, ref mut program_output) in team_outputs.iter_mut() {
                    validate_robot_output(program_output, team, &state.objs);
                }

                let flattened_outputs = team_outputs
                    .into_iter()
                    .map(|(_, output)| output.actions)
                    .flatten()
                    .collect::<HashMap<Id, ValidatedRobotOutput>>();

                run_turn(&flattened_outputs, &mut turn_state.state);

                turn_cb(&CallbackInput {
                    state: turn_state.clone(),
                    logs,
                    robot_outputs: flattened_outputs,
                });
            },
            errored @ ((_, Err(_)), _) | errored @ (_, (_, Err(_))) => {
                let mut errors = HashMap::new();
                let winner = match errored {
                    ((t1, Err(e1)), (t2, Err(e2))) => {
                        errors.insert(t1, e1);
                        errors.insert(t2, e2);
                        None
                    },
                    ((t1, Err(e1)), (t2, Ok(_))) => {
                        errors.insert(t1, e1);
                        Some(t2)
                    },
                    ((t1, Ok(_)), (t2, Err(e2))) => {
                        errors.insert(t2, e2);
                        Some(t1)
                    },
                };
                turn_cb(&CallbackInput { state: turn_state.clone(), logs, ..Default::default() });
                return MainOutput { winner, errors }
            }
        }
    }
    let winner = turn_state.state.determine_winner();
    MainOutput { winner, errors: HashMap::new() }
}

fn run_turn(robot_outputs: &HashMap<Id, ValidatedRobotOutput>, state: &mut State) {
    let mut movement_map = MultiMap::new();
    let mut attack_map = MultiMap::new();

    for (id, action) in robot_outputs.iter() {
        let map = match action.type_ {
            ActionType::Move => &mut movement_map,
            ActionType::Attack => &mut attack_map,
        };
        let obj = state.objs.get(&id).unwrap();
        map.insert(obj.coords() + action.direction, id);
    }

    let movement_grid = movement_map
        .iter()
        .filter_map(|(coords, id)| {
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
                state.objs.get_mut(id).map(|obj| obj.details())
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
