use multimap::MultiMap;
use rand::Rng;

pub mod types;

include!("./types.rs");

const TEAMS: [Team; 2] = [Team::Red, Team::Blue];

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
    const UNIT_HEALTH: usize = 10;
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
        let mut objs: ObjMap = TEAMS
            .iter()
            .map(|team| Self::create_unit_objs(&grid, grid_size, *team))
            .flatten()
            .collect();
        objs.extend(terrain_objs);

        // update the map with the units
        Self::update_grid_map(&mut grid, &objs);

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

    fn update_grid_map(grid: &mut GridMap, objs: &ObjMap) {
        objs.values().for_each(|Obj(basic, _)| {
            if !grid.contains_key(&basic.coords) {
                grid.insert(basic.coords, basic.id);
            }
        })
    }

    fn create_unit_objs(grid: &GridMap, grid_size: usize, team: Team) -> ObjMap {
        (0..Self::TEAM_UNIT_NUM)
            .map(|_| {
                let obj = Obj::new_unit(
                    UnitType::Soldier,
                    Self::random_grid_loc(grid, grid_size),
                    team,
                );
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

    fn determine_winner(self) -> Team {
        let teams = Self::create_team_map(&self.objs);
        teams
            .into_iter()
            .max_by_key(|(_, ids)| ids.len())
            .unwrap()
            .0
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

pub fn run<Err, RunF, TurnCb, FinishCb>(
    run_team_f: RunF,
    turn_cb: TurnCb,
    finish_cb: FinishCb,
    max_turn: usize,
) -> Result<(), Err>
where
    RunF: Fn(Team, RobotInput) -> Result<RobotOutput, Err>,
    TurnCb: Fn(&TurnState) -> (),
    FinishCb: Fn(MainOutput) -> (),
{
    let state = State::new(MapType::Rect, GRID_SIZE);
    let mut turn_state = TurnState { turn: 0, state };
    while turn_state.turn != max_turn {
        let team_outputs = TEAMS
            .iter()
            .map(|team| {
                Ok((
                    *team,
                    run_team_f(*team, RobotInput::new(turn_state.clone(), *team, GRID_SIZE))?,
                ))
            })
            .collect::<Result<HashMap<Team, RobotOutput>, _>>()?;

        run_turn(team_outputs, &mut turn_state);

        turn_cb(&turn_state);
    }
    finish_cb(MainOutput {
        winner: turn_state.state.determine_winner(),
    });
    Ok(())
}

fn run_turn(team_outputs: HashMap<Team, RobotOutput>, turn_state: &mut TurnState) {
    team_outputs
        .iter()
        .for_each(|(team, output)| output.verify(*team, &turn_state.state.objs));

    let all_actions = team_outputs
        .into_iter()
        .map(|(_, output)| output.actions)
        .flatten()
        .collect::<ActionMap>();
    let (move_actions, attack_actions) = all_actions
        .into_iter()
        .partition::<ActionMap, _>(|(_, action)| action.type_ == ActionType::Move);

    let movement_map = get_multimap_from_action_map(&turn_state.state.objs, move_actions);
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

    turn_state
        .state
        .grid
        .retain(|coords, _| !movement_grid.contains_key(coords));
    update_grid_with_movement(
        &turn_state.state.objs,
        &mut turn_state.state.grid,
        movement_grid,
    );

    get_multimap_from_action_map(&turn_state.state.objs, attack_actions)
        .iter_all()
        .for_each(|(coords, attacks)| {
            let attack_power = attacks.len() * Obj::ATTACK_POWER;
            let id = turn_state.state.grid.get(coords).unwrap();
            if let Obj(_, ObjDetails::Unit(ref mut unit)) =
                turn_state.state.objs.get_mut(id).unwrap()
            {
                unit.health = unit.health.saturating_sub(attack_power);
                if unit.health == 0 {
                    turn_state.state.objs.remove(id);
                    turn_state.state.grid.remove(coords);
                }
            }
        });

    turn_state.turn += 1;
}

pub fn get_multimap_from_action_map(objs: &ObjMap, actions: ActionMap) -> MultiMap<Coords, Id> {
    actions
        .into_iter()
        .map(|(id, action)| {
            let Obj(basic, _) = objs.get(&id).unwrap();
            (basic.coords + action.direction, id)
        })
        .collect()
}

pub fn update_grid_with_movement(objs: &ObjMap, grid: &mut GridMap, movement_grid: GridMap) {
    let (illegal_moves, legal_moves): (GridMap, GridMap) = movement_grid
        .into_iter()
        .partition(|(coords, _)| grid.contains_key(coords));

    if illegal_moves.is_empty() {
        grid.extend(legal_moves)
    } else {
        // insert the units with illegal moves back in their original location
        illegal_moves.into_iter().for_each(|(_, id)| {
            grid.insert(objs.get(&id).unwrap().0.coords, id);
        });
        update_grid_with_movement(objs, grid, legal_moves);
    }
}
