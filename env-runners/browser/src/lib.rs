use js_sys::{Function as JsFunction, Promise, Uint8Array};
use logic::ProgramError;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{future_to_promise, JsFuture};

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
    pub type WasiRunner;
    #[wasm_bindgen(method, getter)]
    fn init_result(this: &WasiRunner) -> Promise;
    #[wasm_bindgen(method)]
    fn run_turn(this: &WasiRunner, input: &[u8]) -> Promise;

    pub type WasiResult;
    #[wasm_bindgen(method, getter)]
    fn logs(this: &WasiResult) -> String;
    #[wasm_bindgen(method, getter)]
    fn output(this: &WasiResult) -> Vec<u8>;
}

struct JsRunner {
    runner: WasiRunner,
}

impl JsRunner {
    async fn new(runner: WasiRunner) -> Result<Self, ProgramError> {
        let res = JsFuture::from(runner.init_result()).await.map_err(|err| {
            dbg!(err);
            ProgramError::InternalError
        })?;
        let res = res
            .dyn_into::<Uint8Array>()
            .map_err(|_| ProgramError::InternalError)?
            .to_vec();
        let init_result: Result<(), ProgramError> = serde_json::from_slice(&res)?;

        init_result.map(|()| JsRunner { runner })
    }
}

#[async_trait::async_trait(?Send)]
impl logic::RobotRunner for JsRunner {
    async fn run(&mut self, input: logic::ProgramInput) -> logic::RunnerResult {
        let input = serde_json::to_vec(&input)?;
        let result = JsFuture::from(self.runner.run_turn(&input))
            .await
            .map_err(|err| {
                dbg!(err);
                ProgramError::InternalError
            })?
            .unchecked_into::<WasiResult>();

        let logs = result.logs();
        let output = result.output();

        let mut output: logic::ProgramOutput = serde_json::from_slice(&output)?;
        output.logs.extend(logs.split('\n').map(ToOwned::to_owned));
        Ok(output)
    }
}

#[wasm_bindgen]
pub fn run(
    runner1: WasiRunner,
    runner2: WasiRunner,
    turn_callback: JsFunction,
    turn_num: usize,
) -> Promise {
    future_to_promise(async move {
        let output = logic::run(
            JsRunner::new(runner1).await,
            JsRunner::new(runner2).await,
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