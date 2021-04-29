#![allow(non_snake_case)]
#![type_length_limit = "1526423"]

use rusoto_core::Region;
use rusoto_sqs::{SendMessageRequest, Sqs, SqsClient};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use logic::{ProgramError, Team};
use native_runner::TokioRunner;
use tokio::time::{self, Duration, Instant};
use tokio::{io, task};

use wasi_process::WasiProcess;
use wasmer_wasi::{WasiState, WasiVersion};

use base64::write::EncoderWriter as Base64Writer;
use flate2::write::GzEncoder;
use flate2::Compression;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

/*
SAMPLE EVENT
{
    "Records": Array([Object({
        "attributes": Object({
            "ApproximateFirstReceiveTimestamp": String("1523232000001"),
            "ApproximateReceiveCount": String("1"),
            "SenderId": String("123456789012"),
            "SentTimestamp": String("1523232000000")
        }),
        "awsRegion": String("us-east-1"),
        "body": String("{\"r1_id\": 1, \"r1_code\": \"\", \"r2_id\": 2, \"r2_code\":\"\"}"),
        "eventSource": String("aws:sqs"),
        "eventSourceARN": String("arn:aws:sqs:us-east-1:123456789012:MyQueue"),
        "md5OfBody": String("7b270e59b47ff90a553787216d55d91d"),
        "messageAttributes": Object({}),
        "messageId": String("19dd0b57-b21e-4ac1-bd88-01bbb068cb78"),
        "receiptHandle": String("MessageReceiptHandle")
    })])
}
*/

#[derive(Deserialize, Serialize, Debug)]
struct LambdaInput {
    Records: Vec<LambdaInputRecord>,
}

#[derive(Deserialize, Serialize, Debug)]
struct LambdaInputRecord {
    #[serde(with = "serde_with::json::nested")]
    body: Input,
}

#[derive(Deserialize, Serialize, Debug)]
struct Input {
    turn_num: usize,
    r1_id: usize,
    pr1_id: usize,
    r1_code: String,
    r1_lang: Lang,
    r2_id: usize,
    pr2_id: usize,
    r2_code: String,
    r2_lang: Lang,
    board_id: usize,
}

#[derive(Serialize, Debug)]
enum OutputTeam {
    R1,
    R2,
}

#[derive(Serialize, Debug)]
struct Output {
    r1_id: usize,
    pr1_id: usize,
    r1_time: f64,
    r2_id: usize,
    pr2_id: usize,
    r2_time: f64,
    #[serde(with = "serde_with::json::nested")]
    data: logic::MainOutput,
    winner: Option<OutputTeam>,
    errored: bool,
    board_id: usize,
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

// TODO: deduplicate with cli somehow
#[derive(Copy, Clone, Deserialize, Serialize, Debug)]
enum Lang {
    Python,
    Javascript,
}

static STORE: Lazy<wasmer::Store> = Lazy::new(|| {
    let engine = wasmer::JITEngine::headless();
    // let seed = rand::random();
    // engine.set_deterministic_prefixer(move |bytes| {
    //     let mut hasher = crc32fast::Hasher::new_with_initial(seed);
    //     hasher.update(bytes);
    //     format!("{:08x}", hasher.finalize())
    // });
    wasmer::Store::new(&engine)
});

impl Lang {
    fn get_wasm(self) -> (&'static wasmer::Module, WasiVersion) {
        macro_rules! load_cache {
            ($name:literal) => {{
                static MODULE: Lazy<(wasmer::Module, WasiVersion)> = Lazy::new(|| {
                    let artifact_path = concat!("/opt/wasmer-cache/", $name);
                    let module = unsafe {
                        wasmer::Module::deserialize_from_file(&STORE, artifact_path)
                            .expect("couldn't load module from cache")
                    };
                    let version = wasmer_wasi::get_wasi_version(&module, false)
                        .unwrap_or(WasiVersion::Latest);
                    (module, version)
                });
                let (ref m, v) = *MODULE;
                (m, v)
            }};
        }
        match self {
            Self::Python => load_cache!("pyrunner.wjit"),
            Self::Javascript => load_cache!("jsrunner.wjit"),
        }
    }
}

// from cli/main.rs -- TODO: deduplicate
fn make_sourcedir_inline(source: &str) -> tempfile::TempDir {
    let sourcedir = tempfile::tempdir().expect("couldn't create temporary directory");
    std::fs::write(sourcedir.path().join("sourcecode"), source)
        .expect("Couldn't write code to disk");
    sourcedir
}

fn make_state(code: &str) -> (WasiState, tempfile::TempDir) {
    let tempdir = make_sourcedir_inline(code);
    let mut state = WasiState::new("robot");
    wasi_process::add_stdio(&mut state);
    let state = state
        .preopen(|p| p.directory(&tempdir).alias("source").read(true))
        .expect("preopen failed")
        .arg("/source/sourcecode")
        .build()
        .unwrap();
    (state, tempdir)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _sentry = sentry::init(std::env::var("SENTRY_DSN").unwrap());
    let func = lambda::handler_fn(run);
    lambda::run(func).await.unwrap();
    Ok(())
}

async fn run(data: LambdaInput, _ctx: lambda::Context) -> Result<(), Error> {
    println!("DATA RECEIVED: {}", serde_json::to_string(&data)?);

    let input_data = data.Records.into_iter().next().unwrap().body;

    println!(
        "LANGS: {:?} vs {:?}",
        input_data.r1_lang, input_data.r2_lang
    );

    let make_runner = |code, lang: Lang| {
        let (module, version) = lang.get_wasm();
        let (state, sourcedir) = make_state(code);
        let env = wasmer_wasi::WasiEnv::new(state);
        let imports = wasmer_wasi::generate_import_object_from_env(&STORE, env, version);
        let instance = wasmer::Instance::new(&module, &imports).unwrap();
        let mut proc = WasiProcess::new(&instance, Default::default()).expect("modules have start");
        let stdin = io::BufWriter::new(proc.stdin.take().unwrap());
        let stdout = io::BufReader::new(proc.stdout.take().unwrap());
        proc.stdout.take();
        let t = task::spawn(async move {
            let start_t = Instant::now();
            let res = match time::timeout(TIMEOUT, proc).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(_wasm_err)) => Err(ProgramError::InternalError),
                Err(_timeout) => Err(ProgramError::Timeout(TIMEOUT)),
            };
            (start_t.elapsed(), res)
        });
        async move { (TokioRunner::new(stdin, stdout).await, t, sourcedir) }
    };

    let ((r1, t1, _d1), (r2, t2, _d2)) = tokio::join!(
        make_runner(&input_data.r1_code, input_data.r1_lang),
        make_runner(&input_data.r2_code, input_data.r2_lang),
    );

    let runners = maplit::btreemap! {
        Team::Blue => r1,
        Team::Red => r2,
    };
    let run_fut = logic::run(runners, |_| {}, input_data.turn_num);

    let (mut output, err1, err2) = tokio::join!(run_fut, t1, t2);

    let mut handle_res = |team, res: Result<_, task::JoinError>| match res {
        Ok((dur, res)) => {
            if let Err(e) = res {
                output.errors.insert(team, e);
            }
            Duration::as_secs_f64(&dur)
        }
        Err(_) => {
            output.errors.insert(team, ProgramError::InternalError);
            -1.0
        }
    };
    let r1_time = handle_res(Team::Red, err1);
    let r2_time = handle_res(Team::Blue, err2);

    let winner = match output.winner {
        Some(Team::Blue) => Some(OutputTeam::R1),
        Some(Team::Red) => Some(OutputTeam::R2),
        None => None,
    };
    let errored = !output.errors.is_empty();

    println!(
        "RESULT: r1_time {:?}, r2_time {:?}, winner {:?}, errored {:?}",
        r1_time, r2_time, winner, errored
    );

    let output = Output {
        r1_id: input_data.r1_id,
        pr1_id: input_data.pr1_id,
        r1_time,
        r2_id: input_data.r2_id,
        pr2_id: input_data.pr2_id,
        r2_time,
        data: output,
        winner,
        errored,
        board_id: input_data.board_id,
    };

    let mut message_body = Vec::<u8>::new();
    {
        let mut b64_enc = Base64Writer::new(&mut message_body, base64::STANDARD);
        let mut gz_enc = GzEncoder::new(&mut b64_enc, Compression::default());
        serde_json::to_writer(&mut gz_enc, &output)?;
        gz_enc.finish()?;
        b64_enc.finish()?;
    }
    let message_body = String::from_utf8(message_body)?;

    let client = SqsClient::new(Region::UsEast1);

    let out_queue_url = std::env::var("BATTLE_QUEUE_OUT_URL")
        .expect("\"BATTLE_QUEUE_OUT_URL\" environmental variable not found");

    client
        .send_message(SendMessageRequest {
            queue_url: out_queue_url,
            message_body,
            delay_seconds: None,
            message_attributes: None,
            message_deduplication_id: None,
            message_group_id: None,
            message_system_attributes: None,
        })
        .await?;

    Ok(())
}

const TIMEOUT: Duration = Duration::from_secs(60);
