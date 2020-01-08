open Base
open Logic_t
open Lwt.Infix

let ( >> ) f g x = f (g x)
let identity x = x
let team_names = ["blue"; "red"]
let letters = String.to_array "abcdefghijklmnopqrstuvwxyz"
let id_length = 5

let generate_id () =
  String.init id_length ~f:(fun _ -> Array.random_element_exn letters)

let log sexp = Sexp.to_string sexp |> Caml.print_endline
let create_basic_obj coords = {coords; id= generate_id ()}
let create_terrain type_ coords = (create_basic_obj coords, Terrain {type_})
let unit_health = 5

let create_unit type_ coords team =
  (create_basic_obj coords, Unit {type_; health= unit_health; team})

module Coords = struct
  module T = struct type t = int * int [@@deriving compare, sexp_of] end
  include T
  include Comparator.Make (T)

  let equal (x1, y1) (x2, y2) = x1 = x2 && y1 = y2
  let log_map v_sexp map = Map.sexp_of_m__t (module T) v_sexp map |> log
end

module Map = struct
  include Map

  let update_exn map key ~f =
    Map.update map key ~f:(function
      | Some v -> f v
      | None -> failwith "Key for Map.update not found" )

  let merge_exn map1 map2 =
    Map.fold map1 ~init:map2 ~f:(fun ~key ~data acc ->
        Map.add_exn acc ~key ~data )

  let partition_tf_keys map ~f =
    Map.partitioni_tf map ~f:(fun ~key ~data:_ -> f key)
end

let create_grid size =
  List.init size ~f:(fun x -> List.init size ~f:(fun y -> (x, y)))
  |> List.concat

let filter_empty type_ size =
  List.filter ~f:(fun (x, y) ->
      match type_ with
      | Circle ->
          let radius = size / 2 in
          let is_wall x y =
            ((x - radius) ** 2) + ((y - radius) ** 2) > radius ** 2
          in
          is_wall x y
      | Rect -> x = 0 || x = size - 1 || y = 0 || y = size - 1 )

let create_map_vars map =
  let terrains = List.map map ~f:(create_terrain Wall) in
  let map =
    List.map terrains ~f:(fun (base, _terrain) -> (base.coords, base.id))
    |> Map.of_alist_exn (module Coords)
  in
  (terrains, map)

let create_map type_ size =
  create_grid size |> filter_empty type_ size |> create_map_vars

let rec random_loc map size =
  let x = Random.int size and y = Random.int size in
  match Map.find map (x, y) with
  | None -> (x, y)
  | Some _ -> random_loc map size

let create_teams objs team_names =
  let init =
    List.map team_names ~f:(fun team -> (team, []))
    |> Map.of_alist_exn (module String)
  in
  List.fold (Map.data objs) ~init ~f:(fun acc (_base, details) ->
      match details with
      | Unit unit_ -> Map.add_multi acc ~key:unit_.team ~data:_base.id
      | Terrain _ -> acc )
  |> Map.to_alist

let create_array_map map size =
  Array.init size ~f:(fun x ->
      Array.init size ~f:(fun y -> Map.find map (x, y)) )

let lwt_join = function
  | [a; b] -> Lwt.both a b >|= fun (a, b) -> [a; b]
  | _ -> failwith "Join requires two arguments."

let check_actions team actions objs =
  List.iter actions ~f:(fun (id, _action) ->
      match Map.find objs id with
      | Some (_base, Unit unit_) ->
          if String.(unit_.team <> team) then
            failwith "Action ID belongs to opposing team."
          else ()
      | Some (_base, Terrain _) -> failwith "Action ID belongs to terrain"
      | None -> failwith "Action ID does not exist." )

let compute_coords (x, y) direction =
  match direction with
  | Left -> (x - 1, y)
  | Right -> (x + 1, y)
  | Up -> (x, y - 1)
  | Down -> (x, y + 1)

let determine_winner (state : turn_state) =
  let teams =
    create_teams (Map.of_alist_exn (module String) state.objs) team_names
  in
  let res =
    List.max_elt teams ~compare:(fun (_, units1) (_, units2) ->
        List.length units1 - List.length units2 )
  in
  match res with
  | Some (team, _) -> team
  | None -> Caml.fst @@ List.hd_exn teams

let get_obj_coords objs id =
  let base, _ = Map.find_exn objs id in
  base.coords

let get_action_target objs id action =
  compute_coords (get_obj_coords objs id) action.direction

let rec validate_movement_map movement_map map objs =
  let conflicting_moves, movement_map =
    Map.partition_tf_keys movement_map ~f:(Map.mem map)
  in
  let map =
    Map.fold conflicting_moves ~init:map ~f:(fun ~key:_coords ~data:id map ->
        Map.set map ~key:(get_obj_coords objs id) ~data:id )
  in
  if Map.is_empty conflicting_moves then (movement_map, map)
  else validate_movement_map movement_map map objs

let map_size = 19
let team_unit_num = 6
let attack_strength = 1

let rec run_turn run1 run2 turn_callback max_turn turn objs
    (map : (Coords.t, id, 'a) Map.t) state_list =
  let state = {turn= turn + 1; objs= Map.to_alist objs} in
  let state_list = state :: state_list in
  if turn = max_turn then Lwt.return state_list
  else
    let (_ : unit) = turn_callback state.turn  in
    let input_teams = create_teams objs team_names in
    let input_map = create_array_map map map_size in
    let input_state =
      {basic= state; additional= {teams= input_teams; map= input_map}}
    in
    let inputs =
      List.map team_names ~f:(fun team -> {team; state= input_state})
    in
    let inputs = List.map inputs ~f:Logic_j.string_of_robot_input in
    let inputs =
      match inputs with [a; b] -> [run1 a; run2 b] | _ -> assert false
    in
    inputs |> lwt_join
    >>= fun (result : string list) ->
    let output_list = List.map result ~f:Logic_j.robot_output_of_string in
    let team_output_list = List.zip_exn team_names output_list in
    List.iter team_output_list ~f:(fun (team, output) ->
        check_actions team output.actions objs );
    let all_actions =
      List.concat_map output_list ~f:(fun output -> output.actions)
    in
    let move_actions, attack_actions =
      List.partition_tf all_actions ~f:(fun (_id, action) ->
          match action.type_ with Move -> true | Attack -> false )
    in
    let movement_map =
      List.map move_actions ~f:(fun (id, action) ->
          (get_action_target objs id action, id) )
      |> Map.of_alist_multi (module Coords)
    in
    let movement_map =
      Map.filter_map movement_map ~f:(fun ids ->
          if List.length ids > 1 then None else Some (List.hd_exn ids) )
    in
    let map =
      let ids = Map.data movement_map in
      Map.filter map ~f:(fun id -> not @@ List.mem ids id ~equal:String.equal)
    in
    let movement_map, map = validate_movement_map movement_map map objs in
    let map = Map.merge_exn map movement_map in
    let attack_map =
      List.map attack_actions ~f:(fun (id, action) ->
          (get_action_target objs id action, attack_strength) )
      |> Map.of_alist_multi (module Coords)
      |> Map.map ~f:(List.sum (module Int) ~f:identity)
    in
    let map_with_attack = Map.map map ~f:(fun id -> (id, 0)) in
    let map_with_attack =
      Map.fold attack_map ~init:map_with_attack
        ~f:(fun ~key:coords ~data:attack acc ->
          Map.change acc coords ~f:(function
            | Some (id, attack_) -> Some (id, attack_ + attack)
            | None -> None ) )
    in
    let objs =
      Map.fold map_with_attack ~init:objs
        ~f:(fun ~key:coords ~data:(id, attack) objs ->
          Map.update_exn objs id ~f:(fun (base, details) ->
              ( {base with coords}
              , match details with
                | Unit unit_ -> Unit {unit_ with health= unit_.health - attack}
                | other -> other ) ) )
    in
    let objs, dead_objs =
      Map.partition_tf objs ~f:(fun (_basic, details) ->
          match details with Unit unit_ -> unit_.health > 0 | _ -> true )
    in
    let map =
      Map.fold dead_objs ~init:map ~f:(fun ~key:_ ~data:(basic, _) acc ->
          Map.remove acc basic.coords )
    in
    run_turn run1 run2 turn_callback max_turn (turn + 1) objs map state_list

let start run1 run2 turn_callback max_turn =
  Random.self_init ();
  let terrains, map = create_map Rect map_size in
  let map, units =
    List.fold team_names ~init:(map, []) ~f:(fun acc team ->
        List.fold (List.range 0 team_unit_num) ~init:acc
          ~f:(fun (map, units) _ ->
            let coords = random_loc map map_size in
            let ((basic, _) as unit_) = create_unit Soldier coords team in
            let map = Map.set map ~key:coords ~data:basic.id in
            (map, unit_ :: units) ) )
  in
  let objs =
    List.append units terrains
    |> List.map ~f:(fun ((base, _) as obj) -> (base.id, obj))
    |> Map.of_alist_exn (module String)
  in
  run_turn run1 run2 turn_callback max_turn 0 objs map []
  >|= fun states ->
  {turns= List.rev states; winner= determine_winner @@ List.hd_exn states}
  |> Logic_j.string_of_main_output
