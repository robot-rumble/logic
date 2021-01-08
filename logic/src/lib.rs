use std::cmp::Ordering;
use std::collections::BTreeMap;

use futures_util::future::{join_all, FutureExt};
use multimap::MultiMap;
use rand::seq::SliceRandom;

pub use types::*;

use strum::IntoEnumIterator;

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
    const TEAM_UNIT_NUM: usize = 5;
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
                MapType::Circle => distance_from_center(*coords) >= (size / 2 - 1).pow(2) as i32,
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

    fn spawn_units(&mut self) {
        let Self {
            spawn_points,
            grid,
            objs,
        } = self;
        let mut available_points = spawn_points
            .iter()
            .copied()
            .filter(|loc| !grid.contains_key(loc) && !grid.contains_key(&Self::mirror_loc(*loc)))
            .collect::<Vec<_>>();
        let it = (0..Self::TEAM_UNIT_NUM).flat_map(|_| {
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

    fn create_team_map(objs: &ObjMap) -> TeamMap {
        Team::iter()
            .map(|team| {
                (
                    team,
                    objs.values()
                        .filter_map(|obj| match obj.details() {
                            ObjDetails::Unit(unit) if unit.team == team => Some(obj.id()),
                            _ => None,
                        })
                        .collect(),
                )
            })
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

impl<'a> ProgramInput<'a> {
    pub fn new(turn_state: &'a TurnState, team: Team, grid_size: usize) -> Self {
        let TurnState { turn, ref state } = *turn_state;
        let teams = State::create_team_map(&state.objs);
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

fn handle_program_errors(
    errors: BTreeMap<Team, ProgramError>,
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

const GRID_SIZE: usize = 19;

#[cfg_attr(not(feature = "robot-runner-not-send"), async_trait::async_trait)]
#[cfg_attr(feature = "robot-runner-not-send", async_trait::async_trait(? Send))]
pub trait RobotRunner {
    async fn run(&mut self, input: ProgramInput<'_>) -> ProgramResult;
}

#[cfg(not(feature = "robot-runner-not-send"))]
#[async_trait::async_trait]
impl<F> RobotRunner for F
where
    F: FnMut(ProgramInput) -> ProgramResult + Send,
{
    async fn run(&mut self, input: ProgramInput<'_>) -> ProgramResult {
        (self)(input)
    }
}

#[cfg(feature = "robot-runner-not-send")]
#[async_trait::async_trait(? Send)]
impl<F> RobotRunner for F
where
    F: FnMut(ProgramInput) -> ProgramResult,
{
    async fn run(&mut self, input: ProgramInput<'_>) -> ProgramResult {
        (self)(input)
    }
}

#[inline]
fn unwrap_result_map<T>(
    map: impl Iterator<Item = (Team, Result<T, ProgramError>)>,
    mut ok: impl FnMut(Team, T),
) -> Option<BTreeMap<Team, ProgramError>> {
    let mut errors = BTreeMap::new();
    for (team, res) in map {
        match res {
            Ok(t) => ok(team, t),
            Err(e) => {
                errors.insert(team, e);
            }
        }
    }
    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

// Team 1: Blue, Team 2: Red
pub async fn run<TurnCb, R>(
    runners: BTreeMap<Team, Result<R, ProgramError>>,
    mut turn_cb: TurnCb,
    max_turn: usize,
) -> MainOutput
where
    TurnCb: FnMut(&CallbackInput),
    R: RobotRunner,
{
    let all_teams = runners.keys().copied().collect::<Box<[_]>>();
    let mut run_funcs = BTreeMap::new();
    let errs = unwrap_result_map(runners.into_iter(), |team, runner| {
        run_funcs.insert(team, runner);
    });
    if let Some(errs) = errs {
        return handle_program_errors(errs, &all_teams, vec![]);
    }

    let mut turns = Vec::with_capacity(max_turn);

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
                .run(ProgramInput::new(&turn_state, team, GRID_SIZE))
                .map(move |program_result| (team, program_result))
        }))
        .await;

        let mut merged_actions = BTreeMap::new();
        let mut team_logs = BTreeMap::new();
        let mut team_debug_inspections = BTreeMap::new();
        let mut merged_debug_tables = BTreeMap::new();
        let errs = unwrap_result_map(program_results.into_iter(), |team, output| {
            merged_actions.extend(output.robot_actions.into_iter().map(|(id, action)| {
                (
                    id,
                    validate_robot_action(action, team, id, &turn_state.state.objs),
                )
            }));
            team_logs.insert(team, output.logs);
            team_debug_inspections.insert(team, output.debug_inspections);
            if output
                .debug_tables
                .keys()
                .all(|id| is_id_valid(team, *id, &turn_state.state.objs))
            {
                merged_debug_tables.extend(output.debug_tables)
            }
        });
        if let Some(errs) = errs {
            return handle_program_errors(errs, &all_teams, turns);
        }

        let old_objs = turn_state.state.objs.clone();
        let old_turn = turn_state.turn;

        // update state
        run_turn(&merged_actions, &mut turn_state.state);

        // but the new state isn't passed until the next cycle
        let turn = CallbackInput {
            state: StateForOutput {
                objs: old_objs,
                turn: old_turn,
            },
            robot_actions: merged_actions,
            logs: team_logs,
            debug_inspections: team_debug_inspections,
            debug_tables: merged_debug_tables,
        };
        turn_cb(&turn);
        turns.push(turn);
    }
    let winner = turn_state.state.determine_winner();
    MainOutput {
        winner,
        errors: BTreeMap::new(),
        turns,
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
        map.insert(obj.coords() + action.direction, id);
    }

    let movement_grid = movement_map
        .iter()
        .filter_map(|(coords, &id)| match movement_map.get_vec(coords) {
            Some(vec) if vec.len() > 1 => vec
                .into_iter()
                .map(|id| state.objs.get(&id).unwrap())
                .min_by_key(|obj| {
                    match (
                        coords.0 as isize - obj.coords().0 as isize,
                        coords.1 as isize - obj.coords().1 as isize,
                    ) {
                        (0, 1) => 1,
                        (-1, 0) => 2,
                        (0, -1) => 3,
                        (1, 0) => 4,
                        _ => 10,
                    }
                })
                .map(|obj| (*coords, obj.id())),
            Some(_) => Some((*coords, *id)),
            None => None,
        })
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
