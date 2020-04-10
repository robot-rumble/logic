use itertools::Itertools;
use sqlx::PgPool;
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let pool = PgPool::builder()
        .max_size(5)
        .build(&dotenv::var("DATABASE_URL")?)
        .await?;

    let mut interval = time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let (red, blue) = automatchmake(&pool).await?;
        println!("running {} against {}", red.name, blue.name);
        let winner = tokio::task::spawn_blocking(move || run_python(red, blue)).await?;
        println!("{} won!", winner.name);
    }
}

struct Robot {
    name: String,
    id: i32,
    code: String,
}

async fn automatchmake(pool: &PgPool) -> sqlx::Result<(Robot, Robot)> {
    // TODO: check if they errored in their most recent match
    let output = sqlx::query_as!(
        Robot,
        "SELECT name, id, code FROM robots WHERE automatch = TRUE ORDER BY random() LIMIT 2"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect_tuple()
    .unwrap();
    Ok(output)
}

fn run_python(red: Robot, blue: Robot) -> Robot {
    let vm = &rustpython_vm::VirtualMachine::new(rustpython_vm::PySettings {
        initialization_parameter: rustpython_vm::InitParameter::InitializeInternal,
        ..Default::default()
    });

    let compile = |code: &str| {
        vm.compile(
            code,
            rustpython_compiler::mode::Mode::Exec,
            "<robot>".to_owned(),
        )
        .unwrap()
    };

    let res = vm.unwrap_pyresult(pyrunner::run_python(
        compile(&red.code),
        compile(&blue.code),
        |_| {},
        None::<fn(&str)>,
        20,
        vm,
    ));

    match res.winner {
        logic::Team::Red => red,
        logic::Team::Blue => blue,
    }
}
