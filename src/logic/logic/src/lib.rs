use std::cmp::Ordering;
use std::collections::HashMap;

use multimap::MultiMap;
use rand::Rng;
use strum::IntoEnumIterator;

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
                (obj.0.id, obj)
            })
            .collect()
    }

    fn create_grid_map(objs: &ObjMap) -> GridMap {
        objs.values()
            .map(|Obj(basic, _)| (basic.coords, basic.id))
            .collect()
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
                grid.insert(obj.0.coords, obj.0.id);
                (obj.0.id, obj)
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
            .filter_map(|Obj(basic, details)| match details {
                ObjDetails::Unit(unit) => Some((unit.team, basic.id)),
                _ => None,
            })
            .collect::<MultiMap<Team, Id>>()
            .into_iter()
            .collect()
    }

    fn determine_winner(self) -> Option<Team> {
        let mut reds = 0;
        let mut blues = 0;
        for (_, Obj(_, details)) in self.objs {
            if let ObjDetails::Unit(u) = details {
                match u.team {
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

impl RobotInput {
    pub fn new(turn_state: TurnState, team: Team, grid_size: usize) -> Self {
        let TurnState { turn, state } = turn_state;
        let teams = State::create_team_map(&state.objs);
        Self {
            state: StateForRobotInput {
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

impl RobotOutput {
    pub fn verify(&self, team: Team, objs: &ObjMap) {
        self.actions.keys().for_each(|id| match objs.get(id) {
            Some(Obj(_, ObjDetails::Unit(unit))) if unit.team != team => {
                panic!("Action ID points to unit on other team")
            }
            Some(Obj(_, ObjDetails::Terrain(_))) => panic!("Action ID points to terrain"),
            None => panic!("Action ID points to nonexistent object"),
            _ => (),
        })
    }
}

const GRID_SIZE: usize = 19;

pub fn run<Err, RunF, TurnCb>(
    mut run_team_f: RunF,
    mut turn_cb: TurnCb,
    max_turn: usize,
) -> Result<MainOutput, Err>
where
    RunF: FnMut(Team, RobotInput) -> Result<RobotOutput, Err>,
    TurnCb: FnMut(&TurnState) -> (),
{
    let state = State::new(MapType::Rect, GRID_SIZE);
    let mut turn_state = TurnState { turn: 0, state };
    while turn_state.turn < max_turn {
        let team_outputs = Team::iter()
            .map(|team| {
                Ok((
                    team,
                    run_team_f(team, RobotInput::new(turn_state.clone(), team, GRID_SIZE))?,
                ))
            })
            .collect::<Result<HashMap<Team, RobotOutput>, _>>()?;

        run_turn(team_outputs, &mut turn_state.state);
        turn_state.turn += 1;

        turn_cb(&turn_state);
    }
    let winner = turn_state.state.determine_winner();
    Ok(MainOutput { winner })
}

fn run_turn(team_outputs: HashMap<Team, RobotOutput>, state: &mut State) {
    team_outputs
        .iter()
        .for_each(|(team, output)| output.verify(*team, &state.objs));

    let mut movement_map = MultiMap::new();
    let mut attack_map = MultiMap::new();

    team_outputs
        .into_iter()
        .map(|(_, output)| output.actions)
        .flatten()
        .for_each(|(id, action)| {
            let map = match action.type_ {
                ActionType::Move => &mut movement_map,
                ActionType::Attack => &mut attack_map,
            };
            let Obj(basic, _) = state.objs.get(&id).unwrap();
            map.insert(basic.coords + action.direction, id);
        });

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

    attack_map.iter_all().for_each(|(coords, attacks)| {
        let attack_power = attacks.len() * Obj::ATTACK_POWER;
        let id = match state.grid.get(coords) {
            Some(id) => id,
            None => return,
        };
        if let Some(Obj(_, ObjDetails::Unit(ref mut unit))) = state.objs.get_mut(id) {
            unit.health = unit.health.saturating_sub(attack_power);
            if unit.health == 0 {
                state.objs.remove(id).unwrap();
                state.grid.remove(coords).unwrap();
            }
        }
    });
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
        illegal_moves.into_iter().for_each(|(_, id)| {
            grid.insert(objs.get(&id).unwrap().0.coords, id);
        });
        update_grid_with_movement(objs, grid, legal_moves);
    }
}
