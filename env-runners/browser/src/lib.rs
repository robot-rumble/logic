use js_sys::{Function as JsFunction, Promise, Uint8Array};
use logic::{ProgramError, ProgramResult};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{future_to_promise, JsFuture};

#[wasm_bindgen]
extern "C" {
    #[allow(unused)]
    #[wasm_bindgen(js_namespace = console, js_name = debug)]
    fn console_debug(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(s: JsValue);
}
#[allow(unused)]
macro_rules! dbg {
    ($x:expr) => {{
        let x = $x;
        console_debug(&format!("{} : {:?}", stringify!($x), x));
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
    fn run_turn(this: &WasiRunner, input: Uint8Array) -> Promise;

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
    async fn new(runner: WasiRunner) -> ProgramResult<Self> {
        let res = JsFuture::from(runner.init_result()).await.map_err(|err| {
            console_error(err);
            ProgramError::InternalError
        })?;
        let res = res
            .dyn_into::<Uint8Array>()
            .map_err(|_| ProgramError::InternalError)?
            .to_vec();
        let init_result: ProgramResult<()> = serde_json::from_slice(&res)?;

        init_result.map(|()| JsRunner { runner })
    }
}

#[async_trait::async_trait(?Send)]
impl logic::RobotRunner for JsRunner {
    async fn run(&mut self, input: logic::ProgramInput<'_>) -> ProgramResult {
        let input = serde_json::to_vec(&input)?;
        let result = JsFuture::from(self.runner.run_turn(Uint8Array::from(&*input)))
            .await
            .map_err(|err| {
                console_error(err);
                ProgramError::InternalError
            })?
            .unchecked_into::<WasiResult>();

        let logs = result.logs();
        let output = result.output();

        let mut res: ProgramResult = serde_json::from_slice(&output)?;
        if let Ok(ref mut output) = res {
            if !logs.is_empty() {
                output.logs.extend(logs.split('\n').map(ToOwned::to_owned));
            }
        }
        res
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
        let (r1, r2) = futures_util::join!(JsRunner::new(runner1), JsRunner::new(runner2),);
        let runners = maplit::btreemap! {
            logic::Team::Blue => r1,
            logic::Team::Red => r2,
        };
        let output = logic::run(
            runners,
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
