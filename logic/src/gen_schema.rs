use schemars::{schema_for, JsonSchema};

// hack to reliably generate definitions for both ProgramInput and ProgramResult
#[derive(JsonSchema)]
struct __RRHack(logic::ProgramInput, ProgramResult);

// get a nice name instead of ResultOf_ProgramOutputOr_ProgramError
#[derive(JsonSchema)]
struct ProgramResult(logic::ProgramResult);

fn main() {
    let schema = schema_for!(__RRHack);
    let stdout = std::io::stdout();
    serde_json::to_writer(stdout.lock(), &schema).unwrap();
    println!();
}
