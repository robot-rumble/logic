open Base

let separate_fields basic_field_names fields =
  let basic_fields, other_fields =
    List.partition_tf fields ~f:(fun (field, _data) ->
        List.mem basic_field_names field ~equal:String.equal )
  in
  List.iter basic_field_names ~f:(fun name ->
      match Caml.List.assoc_opt name basic_fields with
      | Some _ -> ()
      | None -> failwith @@ "Missing field: " ^ name );
  (basic_fields, other_fields)

let fail ?m:(message = "") json =
  failwith @@ message ^ "Malformed json: " ^ Yojson.Safe.to_string json

module State = struct
  let basic_field_names = ["id"; "coords"]

  let normalize x =
    match x with
    | `Assoc fields ->
        let basic, additional = separate_fields basic_field_names fields in
        `Assoc [("basic", `Assoc basic); ("additional", `Assoc additional)]
    | malformed -> fail malformed

  let restore x =
    match x with
    | `Assoc [("basic", `Assoc basic); ("additional", `Assoc additional)] ->
        `Assoc (List.append basic additional)
    | malformed -> fail malformed
end

module Obj = struct
  let type_field_name = "_type"
  let basic_field_names = ["winner"; "turns"]

  let modify_nested x modify =
    match x with
    | `Assoc fields -> (
      match Caml.List.assoc_opt "objs" fields with
      | Some (`Assoc objs) ->
          let objs = List.Assoc.map objs ~f:modify in
          let root =
            List.map fields ~f:(fun (field, data) ->
                if String.(field = "objs") then ("objs", `Assoc objs)
                else (field, data) )
          in
          `Assoc root
      | Some _ | None -> failwith "Missing objs field" )
    | malformed -> fail malformed ~m:"nested"

  let normalize x =
    modify_nested x (function
      | `Assoc fields -> (
        match Caml.List.assoc_opt type_field_name fields with
        | Some type_ ->
            let basic, detail = separate_fields basic_field_names fields in
            `List [`Assoc basic; `List [type_; `Assoc detail]]
        | None -> failwith "Missing type field" )
      | malformed -> fail malformed )

  let restore x =
    modify_nested x (function
      | `List [`Assoc basic; `List [type_; `Assoc detail]] ->
          `Assoc (List.append basic detail |> List.cons ("type_", type_))
      | malformed -> fail malformed )
end
