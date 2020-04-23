use js_sys::{Array, Function as JsFunction, Object, Promise, Uint8Array};
use logic::ProgramError;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{future_to_promise, JsFuture};

use std::collections::VecDeque;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn debug(s: &str);
}
macro_rules! dbg {
    ($x:expr) => {{
        let x = $x;
        debug(&format!("{} : {:?}", stringify!($x), x));
        x
    }};
}

#[wasm_bindgen(start)]
pub fn init_global() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
extern "C" {
    pub type WasiWorker;
    #[wasm_bindgen(getter)]
    fn args(this: &WasiWorker) -> Array;
    #[wasm_bindgen(getter)]
    fn env(this: &WasiWorker) -> Object;
    #[wasm_bindgen(method)]
    fn data_promise(this: &WasiWorker) -> Promise;
    #[wasm_bindgen(method)]
    fn write_promise(this: &WasiWorker, chunk: &[u8]) -> Promise;
}

struct JsRunner {
    worker: WasiWorker,
    cached: VecDeque<String>,
}

impl JsRunner {
    async fn new(worker: WasiWorker) -> Result<Self, ProgramError> {
        let runner = JsRunner {
            worker,
            cached: VecDeque::new(),
        };

        Ok(runner)
    }

    async fn read_worker(&mut self) -> Result<Option<String>, ProgramError> {
        if let Some(data) = self.cached.pop_front() {
            return Ok(Some(data));
        }
        JsFuture::from(self.worker.data_promise())
            .await
            .ok()
            .and_then(|res| {
                res.dyn_ref::<Uint8Array>()
                    .map(|b| String::from_utf8(b.to_vec()).ok())
            })
            .ok_or(ProgramError::InternalError)
    }

    async fn read_line(&mut self) -> Result<Option<String>, ProgramError> {
        let mut line = String::new();
        loop {
            let mut data = match self.read_worker().await? {
                Some(d) => d,
                None => return Ok(None),
            };
            if let Some(pos) = data.find('\n') {
                self.cached.push_back(data.split_off(pos));
                line.push_str(&data);
                break;
            }
            line.push_str(&data);
        }
        Ok(Some(line))
    }
}

#[async_trait::async_trait(?Send)]
impl logic::RobotRunner for JsRunner {
    async fn run(&mut self, input: logic::ProgramInput) -> logic::RunnerResult {
        let input = serde_json::to_vec(&input)?;
        JsFuture::from(self.worker.write_promise(&input))
            .await
            .map_err(|err| {
                dbg!(err);
                ProgramError::InternalError
            })?;
        // TODO: deduplicate with the code in cli
        let mut logs = Vec::new();
        let mut output: logic::ProgramOutput = loop {
            let line = self.read_line().await?.ok_or(ProgramError::NoData)?;
            if let Some(output) = strip_prefix(&line, "__rr_output:") {
                break serde_json::from_str(&output)?;
            } else {
                logs.push(line);
            }
        };
        output.logs.extend(logs);
        Ok(output)
    }
}

#[wasm_bindgen]
pub fn run(
    worker1: WasiWorker,
    worker2: WasiWorker,
    turn_callback: JsFunction,
    turn_num: usize,
) -> Promise {
    future_to_promise(async move {
        let output = logic::run(
            JsRunner::new(worker1).await,
            JsRunner::new(worker2).await,
            move |turn_state| {
                turn_callback
                    .call1(
                        &JsValue::UNDEFINED,
                        &JsValue::from_serde(&turn_state).unwrap(),
                    )
                    .expect("Turn callback function failed");
            },
            turn_num,
        )
        .await;
        Ok(JsValue::from_serde(&output).unwrap())
    })
}
fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.starts_with(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}
