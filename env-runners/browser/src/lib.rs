use futures_util::future::{self, FutureExt};
use js_sys::{Function as JsFunction, Promise, Uint8Array};
use logic::{ProgramError, ProgramResult};
use std::time::Duration;
use std::{pin::Pin, task};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{future_to_promise, JsFuture};

#[wasm_bindgen]
extern "C" {
    #[allow(unused)]
    #[wasm_bindgen(js_namespace = console, js_name = debug)]
    fn console_debug(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(s: JsValue);
    #[wasm_bindgen(js_name = clearTimeout)]
    fn clear_timeout(id: u32);
}
#[allow(unused)]
macro_rules! dbg {
    ($x:expr) => {{
        let x = $x;
        console_debug(&format!("{} : {:?}", stringify!($x), x));
        x
    }};
}

#[wasm_bindgen(inline_js = "
export function timeout_promise(secs, id_slot) {
    return new Promise(resolve => {
        id_slot[0] = setTimeout(resolve, secs * 1000);
    });
}
")]
extern "C" {
    fn timeout_promise(secs: f64, id_slot: &mut [u32]) -> Promise;
}

struct Sleep {
    inner: Option<JsFuture>,
    timeout_id: u32,
}
impl std::marker::Unpin for Sleep {}

impl future::Future for Sleep {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<()> {
        if let Some(fut) = &mut self.inner {
            match fut.poll_unpin(cx) {
                task::Poll::Ready(Ok(_)) => self.inner = None,
                task::Poll::Ready(Err(err)) => wasm_bindgen::throw_val(err),
                task::Poll::Pending => return task::Poll::Pending,
            }
        }
        task::Poll::Ready(())
    }
}
impl future::FusedFuture for Sleep {
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}
impl Drop for Sleep {
    fn drop(&mut self) {
        if self.inner.is_some() {
            clear_timeout(self.timeout_id)
        }
    }
}

fn sleep(dur: Duration) -> Sleep {
    let mut id = 0;
    let prom = timeout_promise(dur.as_secs_f64(), std::slice::from_mut(&mut id));
    Sleep {
        inner: Some(JsFuture::from(prom)),
        timeout_id: id,
    }
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

pub struct TimeoutRunner<R: logic::RobotRunner> {
    inner: R,
    timeout: Option<Duration>,
}

impl<R: logic::RobotRunner> TimeoutRunner<R> {
    pub fn new(inner: R, timeout: Option<Duration>) -> Self {
        Self { inner, timeout }
    }
}

#[async_trait::async_trait(?Send)]
impl<R: logic::RobotRunner> logic::RobotRunner for TimeoutRunner<R> {
    async fn run(&mut self, input: logic::ProgramInput<'_>) -> ProgramResult {
        let fut = self.inner.run(input);
        if let Some(dur) = self.timeout {
            futures_util::select_biased! {
                res = fut.fuse() => res,
                () = sleep(dur) => Err(ProgramError::Timeout(dur)),
            }
        } else {
            fut.await
        }
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
            true,
        )
        .await;
        Ok(JsValue::from_serde(&output).unwrap())
    })
}
