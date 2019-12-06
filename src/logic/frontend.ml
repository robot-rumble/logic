open Js_of_ocaml
open Lwt.Infix

class type input =
  object
    method run1 : string -> Js.js_string Js.t Js.meth

    method run2 : string -> Js.js_string Js.t Js.meth

    method turnCallback : int -> unit Js.meth

    method turnNum : int Js.prop
  end

let input_var = "main_input"

let main (input : input Js.t) callback =
  let run1 (robot_input : string) =
    Lwt.wrap (fun () -> input##run1 robot_input |> Js.to_string)
  and run2 (robot_input : string) =
    Lwt.wrap (fun () -> input##run2 robot_input |> Js.to_string)
  and turn_callback turn = input##turnCallback turn in
  Logic.start run1 run2 turn_callback (input##.turnNum - 1)
  >|= Js.string >|= callback

let _ = Js.export "main" main
