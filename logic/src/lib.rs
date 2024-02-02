use std::collections::{BTreeMap, HashMap};

use futures_util::{stream, FutureExt, StreamExt};
use multimap::MultiMap;
use rand::seq::SliceRandom;

pub use types::*;

mod types;

#[inline]
fn binary_remove<T: Ord>(v: &mut Vec<T>, el: &T) {
    let idx = v.binary_search(el).expect("element to remove not in vec");
    v.remove(idx);
}

pub fn new_id() -> Id {
    use std::sync::atomic;
    static COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(1);
    Id(COUNTER.fetch_add(1, atomic::Ordering::Relaxed))
}

fn init_obj_to_obj(InitObj(coords, details): InitObj) -> Obj {
    let basic_obj = BasicObj {
        id: new_id(),
        coords,
    };

    Obj(basic_obj, details)
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

    pub fn id(&self) -> Id {
        self.0.id
    }
    pub fn coords(&self) -> Coords {
        self.0.coords
    }
    pub fn details(&self) -> &ObjDetails {
        &self.1
    }
}

impl State {
    pub fn new(grid_type: MapType, grid_size: usize, settings: Settings) -> Self {
        // create initial objs/map combination
        let (mut objs, spawn_points) = Self::init(grid_type, grid_size);
        let mut grid = Self::create_grid_map(&objs);

        let it = settings
            .grid_init
            .clone()
            .into_iter()
            .map(init_obj_to_obj)
            .inspect(|obj| {
                grid.insert(obj.coords(), obj.id());
            });
        objs.extend(it.map(|obj| (obj.id(), obj)));

        Self {
            objs,
            grid,
            spawn_points,
            settings,
        }
    }

    fn init(type_: MapType, size: usize) -> (ObjMap, Vec<Coords>) {
        let distance_from_center = |Coords(x, y)| {
            ((size / 2) as i32 - x as i32).pow(2) + ((size / 2) as i32 - y as i32).pow(2)
        };

        let grid = (0..size).flat_map(|x| (0..size).map(move |y| Coords(x, y)));
        let objs = grid
            .clone()
            .filter(|&coords| match type_ {
                MapType::Rect => {
                    coords.0 == 0 || coords.0 == size - 1 || coords.1 == 0 || coords.1 == size - 1
                }
                MapType::Circle => distance_from_center(coords) >= (size / 2).pow(2) as i32,
            })
            .map(|coords| {
                let obj = Obj::new_terrain(TerrainType::Wall, coords);
                (obj.id(), obj)
            })
            .collect();
        // spawn_points is sorted, since grid generates coords sorted first by x, then y
        let spawn_points = grid
            .filter(|coords| match type_ {
                MapType::Rect => {
                    coords.0 == 1 || coords.0 == size - 2 || coords.1 == 1 || coords.1 == size - 2
                }
                MapType::Circle => {
                    distance_from_center(*coords) as f32 >= ((size / 2) as f32 - 0.5).powf(2.0)
                }
            })
            .collect();
        (objs, spawn_points)
    }

    fn create_grid_map(objs: &ObjMap) -> GridMap {
        objs.values().map(|obj| (obj.coords(), obj.id())).collect()
    }

    #[inline]
    fn mirror_loc(loc: Coords) -> Coords {
        Coords(GRID_SIZE - loc.0 - 1, GRID_SIZE - loc.1 - 1)
    }

    fn clear_spawn(&mut self) {
        let Self {
            grid,
            objs,
            spawn_points,
            settings: _settings,
        } = self;
        for coords in spawn_points.iter() {
            if let Some(id) = grid.get(coords) {
                if let Some(Obj(_, ObjDetails::Unit(_))) = objs.get_mut(id) {
                    objs.remove(id).unwrap();
                    grid.remove(coords).unwrap();
                }
            }
        }
    }

    fn spawn_units(&mut self, is_initial: bool) {
        let Self {
            spawn_points,
            grid,
            objs,
            settings,
        } = self;
        if let Some(spawn_settings) = &settings.spawn_settings {
            let mut available_points = spawn_points
                .iter()
                .copied()
                .filter(|loc| {
                    !grid.contains_key(loc) && !grid.contains_key(&Self::mirror_loc(*loc))
                })
                .collect::<Vec<_>>();

            let unit_num = if is_initial {
                spawn_settings.initial_unit_num
            } else {
                spawn_settings.recurrent_unit_num
            };
            let it = (0..unit_num).flat_map(|_| {
                let point = available_points.choose(&mut rand::thread_rng()).copied();
                let mirrors = point.map(|point| {
                    binary_remove(&mut available_points, &point);
                    let mirror = Self::mirror_loc(point);
                    binary_remove(&mut available_points, &mirror);
                    (point, mirror)
                });
                mirrors.into_iter().flat_map(|(blue_spawn, red_spawn)| {
                    Iterator::chain(
                        std::iter::once((Team::Blue, blue_spawn)),
                        std::iter::once((Team::Red, red_spawn)),
                    )
                    .map(|(team, loc)| {
                        let obj = Obj::new_unit(UnitType::Soldier, loc, team);
                        (obj.id(), obj)
                    })
                })
            });
            let it = it.inspect(|(id, obj)| {
                grid.insert(obj.coords(), *id);
            });
            objs.extend(it);
        }
    }

    fn create_team_map(objs: &ObjMap, all_teams: &[Team]) -> TeamMap {
        let mut map: TeamMap = all_teams.iter().map(|&team| (team, Vec::new())).collect();
        for obj in objs.values() {
            if let ObjDetails::Unit(unit) = obj.details() {
                map.entry(unit.team).or_default().push(obj.id())
            }
        }
        map
    }

    fn determine_winner(self) -> Option<Team> {
        let mut units_count = BTreeMap::new();
        // the highest score any of the teams have
        let mut max = 0;
        for (_, obj) in self.objs {
            if let ObjDetails::Unit(unit) = obj.details() {
                let count = units_count.entry(unit.team).or_insert(0);
                *count += 1;
                if *count > max {
                    max = *count
                }
            }
        }
        // find the team that has the high score
        let mut winners = units_count.into_iter().filter(|(_, c)| *c == max);
        let mut winner = winners.next();
        // if there are multiple teams tied for `max` score, no-one wins
        if winners.next().is_some() {
            winner = None
        }
        winner.map(|(team, _)| team)
    }
}

impl<'a> ProgramInput<'a> {
    pub fn new(
        turn_state: &'a TurnState,
        all_teams: &[Team],
        team: Team,
        grid_size: usize,
    ) -> Self {
        let TurnState { turn, ref state } = *turn_state;
        let teams = State::create_team_map(&state.objs, all_teams);
        Self {
            state: StateForProgramInput {
                turn,
                objs: (&state.objs).into(),
                grid: (&state.grid).into(),
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
        .and_then(|action| {
            let err_msg = match objs.get(&id).map(|obj| obj.details()) {
                Some(ObjDetails::Unit(unit)) if unit.team != team => {
                    "Action ID points to unit on other team"
                }
                Some(ObjDetails::Terrain(_)) => "Action ID points to terrain",
                None => "Action ID points to nonexistent object",
                _ => return Ok(action),
            };
            Err(RobotErrorAfterValidation::InvalidAction(err_msg.to_owned()))
        })
}

fn is_id_valid(team: Team, id: Id, objs: &ObjMap) -> bool {
    match objs.get(&id).map(|obj| obj.details()) {
        Some(ObjDetails::Unit(unit)) => unit.team == team,
        _ => false,
    }
}

type ErrorMap = BTreeMap<Team, ProgramError>;

fn handle_program_errors(
    errors: ErrorMap,
    all_teams: &[Team],
    turns: Vec<CallbackInput>,
) -> MainOutput {
    let mut winner = Some(None);
    for team in all_teams {
        if !errors.contains_key(team) {
            // `team` didn't error
            // try to declare `team` the winner, but not if someone else is already "winner" - then
            // it's a tie w/ no winner
            winner = if let Some(None) = winner {
                Some(Some(*team))
            } else {
                None
            };
        }
    }
    MainOutput {
        winner: winner.flatten(),
        errors,
        turns,
    }
}

pub const GRID_SIZE: usize = 19;

#[cfg_attr(not(feature = "robot-runner-not-send"), async_trait::async_trait)]
#[cfg_attr(feature = "robot-runner-not-send", async_trait::async_trait(? Send))]
pub trait RobotRunner {
    async fn run(&mut self, input: ProgramInput<'_>) -> ProgramResult;
}

#[inline]
fn check_runner_error<T>(
    errors: &mut ErrorMap,
    team: Team,
    result: Result<T, ProgramError>,
) -> Option<T> {
    match result {
        Ok(t) if errors.is_empty() => Some(t),
        Ok(_) => None,
        Err(e) => {
            errors.insert(team, e);
            None
        }
    }
}

pub async fn run<TurnCb, R>(
    runners: BTreeMap<Team, Result<R, ProgramError>>,
    mut turn_cb: TurnCb,
    max_turn: usize,
    dev_mode: bool,
    settings_option: Option<Settings>,
) -> MainOutput
where
    TurnCb: FnMut(&CallbackInput),
    R: RobotRunner,
{
    let settings = settings_option.unwrap_or_default();

    // all_teams is the list of all the teams participating in the battle
    let all_teams = runners.keys().copied().collect::<Box<[_]>>();
    let all_teams = &*all_teams;

    let mut run_funcs = BTreeMap::new();
    let mut errors = ErrorMap::new();
    for (team, res) in runners {
        if let Some(f) = check_runner_error(&mut errors, team, res) {
            run_funcs.insert(team, f);
        }
    }
    if !errors.is_empty() {
        return handle_program_errors(errors, all_teams, vec![]);
    }

    let mut turns = Vec::with_capacity(max_turn);

    let mut turn_state = TurnState {
        turn: 1,
        state: State::new(MapType::Circle, GRID_SIZE, settings.clone()),
    };
    while turn_state.turn <= max_turn {
        if let Some(spawn_settings) = &settings.spawn_settings {
            if turn_state.turn == 1 {
                turn_state.state.spawn_units(true);
            } else if spawn_settings.spawn_every != 0 {
                if (turn_state.turn - 1) % spawn_settings.spawn_every == 0 {
                    turn_state.state.clear_spawn();
                    turn_state.state.spawn_units(false);
                }
            }
        }

        let runners = run_funcs.iter_mut().map(|(&t, r)| (t, r));
        let turn = match get_turn_data(runners, all_teams, &turn_state, dev_mode).await {
            Ok(t) => t,
            Err(errors) => return handle_program_errors(errors, all_teams, turns),
        };

        // update state
        run_turn(&turn.robot_actions, &mut turn_state.state);

        // but the new state isn't passed until the next cycle
        turn_cb(&turn);
        turns.push(turn);

        turn_state.turn += 1;
    }
    let winner = turn_state.state.determine_winner();
    MainOutput {
        winner,
        errors: BTreeMap::new(),
        turns,
    }
}

async fn get_turn_data<'r, R: RobotRunner + 'r>(
    runners: impl Iterator<Item = (Team, &'r mut R)>,
    all_teams: &[Team],
    turn_state: &TurnState,
    dev_mode: bool,
) -> Result<CallbackInput, ErrorMap> {
    let mut errors = ErrorMap::new();

    let mut turn = CallbackInput {
        state: StateForOutput {
            objs: turn_state.state.objs.clone(),
            turn: turn_state.turn,
        },
        robot_actions: BTreeMap::new(),
        logs: BTreeMap::new(),
        debug_locate_queries: BTreeMap::new(),
        debug_inspect_tables: BTreeMap::new(),
    };

    let mut results: stream::FuturesUnordered<_> = runners
        .map(|(team, runner)| {
            runner
                .run(ProgramInput::new(&turn_state, all_teams, team, GRID_SIZE))
                .map(move |program_result| (team, program_result))
        })
        .collect();

    while let Some((team, result)) = results.next().await {
        let runner_output = match check_runner_error(&mut errors, team, result) {
            Some(o) => o,
            None => continue,
        };
        turn.robot_actions
            .extend(runner_output.robot_actions.into_iter().map(|(id, action)| {
                (
                    id,
                    validate_robot_action(action, team, id, &turn_state.state.objs),
                )
            }));
        turn.logs.insert(team, runner_output.logs);
        if dev_mode {
            turn.debug_locate_queries
                .insert(team, runner_output.debug_locate_queries);
            if runner_output
                .debug_inspect_tables
                .keys()
                .all(|id| is_id_valid(team, *id, &turn_state.state.objs))
            {
                turn.debug_inspect_tables
                    .extend(runner_output.debug_inspect_tables);
            }
        }
    }

    if errors.is_empty() {
        Ok(turn)
    } else {
        Err(errors)
    }
}

fn run_turn(robot_actions: &BTreeMap<Id, ValidatedRobotAction>, state: &mut State) {
    let mut movement_map = MultiMap::new();
    let mut attack_map = MultiMap::new();

    for (id, action) in robot_actions.iter().filter_map(|(id, action)| {
        action
            .as_ref()
            .ok()
            .and_then(|maybe_a| maybe_a.map(|a| (id, a)))
    }) {
        let map = match action.type_ {
            ActionType::Move => &mut movement_map,
            ActionType::Attack => &mut attack_map,
        };
        let obj = state.objs.get(&id).unwrap();
        map.insert(obj.coords() + action.direction, (*id, action.direction));
    }

    let movement_grid = movement_map
        .iter_all()
        .filter_map(|(coords, robots)| {
            let robot_chosen_to_move = if robots.len() > 1 {
                robots.iter().min_by_key(|(_, direction)| match direction {
                    Direction::North => 1,
                    Direction::East => 2,
                    Direction::South => 3,
                    Direction::West => 4,
                })
            } else {
                robots.get(0)
            };
            robot_chosen_to_move.map(|r| (coords, r))
        })
        .collect::<HashMap<_, _>>();

    let movement_grid = movement_grid
        .into_iter()
        .filter(|(&coords, &(_, direction))| {
            let origin_coords = coords + direction.opposite();
            match movement_map.get(&origin_coords) {
                Some((_, direction2)) => direction != direction2.opposite(),
                None => true,
            }
        })
        .map(|(&coords, &(id, _))| (coords, id))
        .collect::<GridMap>();

    state
        .grid
        .retain(|_, id| !movement_grid.values().any(|movement_id| id == movement_id));
    update_grid_with_movement(&mut state.objs, &mut state.grid, movement_grid);

    for (coords, attacks) in attack_map.iter_all() {
        let attack_power = attacks.len() * Obj::ATTACK_POWER;
        if let Some(id) = state.grid.get(coords) {
            if let Some(Obj(_, ObjDetails::Unit(unit))) = state.objs.get_mut(id) {
                unit.health = unit.health.saturating_sub(attack_power);
                if unit.health == 0 {
                    state.objs.remove(id).unwrap();
                    state.grid.remove(coords).unwrap();
                }
            }
        }
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
