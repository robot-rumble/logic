macro_rules! lang_runner {
    ($initfn:path) => {
        fn main() {
            lang_main($initfn);
            use ::logic::{ProgramError, ProgramInput, ProgramOutput};
            fn lang_main<F: FnMut(ProgramInput) -> ProgramOutput>(
                init: fn(&str) -> Result<F, ProgramError>,
            ) {
                use std::io::prelude::*;
                let source_path = std::env::args_os().nth(1).unwrap();
                let source = std::fs::read_to_string(source_path).unwrap();

                let (run_turn, init_result) = match init(&source) {
                    Ok(f) => (Some(f), Ok(())),
                    Err(e) => (None, Err(e)),
                };

                {
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    stdout.write(b"__rr_init:").unwrap();
                    serde_json::to_writer(&mut stdout, &init_result).unwrap();
                    stdout.write(b"\n").unwrap();
                    stdout.flush().unwrap();
                }

                let mut run_turn = run_turn.unwrap_or_else(|| std::process::exit(1));

                let stdin = std::io::stdin();
                for input in stdin.lock().lines() {
                    let input = input.expect("couldn't read input");
                    let input = serde_json::from_str(&input).expect("bad input given to lang runner");
                    let output = run_turn(input);
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    stdout.write(b"__rr_output:").unwrap();
                    serde_json::to_writer(&mut stdout, &output).unwrap();
                    stdout.write(b"\n").unwrap();
                    stdout.flush().unwrap();
                }
            }
        }
        const _: () = {
            use std::cell::RefCell;
            thread_local! {
                static CLOSURE: RefCell<Option<Box<dyn FnMut($crate::ProgramInput) -> $crate::ProgramOutput>>> = RefCell::default();
                static IO_MEM: RefCell<Vec<u8>> = RefCell::default();
            };
            #[export_name = "__rr_io_addr"]
            pub extern "C" fn rr_io_addr() -> *mut u8 {
                IO_MEM.with(|c| c.borrow_mut().as_mut_ptr())
            }
            #[export_name = "__rr_prealloc"]
            pub extern "C" fn rr_prealloc(len: usize) -> *mut u8 {
                IO_MEM.with(|mem| {
                    let mut mem = mem.borrow_mut();
                    mem.clear();
                    mem.resize(len, b'\0');
                    mem.as_mut_ptr()
                })
            }
            fn with_mem(f: impl FnOnce(&mut Vec<u8>)) -> usize {
                IO_MEM.with(|mem| {
                    let mut mem = mem.borrow_mut();
                    f(&mut mem);
                    mem.len()
                })
            }
            #[export_name = "__rr_init"]
            pub extern "C" fn robot_init() -> usize {
                with_mem(|mem| {
                    let source = std::str::from_utf8(&mem).expect("non-utf8 source code");
                    let res: Result<(), $crate::ProgramError> = $initfn(source).map(|closure| {
                        CLOSURE.with(|c| {
                            let mut c = c.borrow_mut();
                            if c.is_some() {
                                panic!("double init");
                            }
                            *c = Some(Box::new(closure));
                        });
                    });
                    mem.clear();
                    serde_json::to_writer(mem, &res).unwrap();
                })
            }
            #[export_name = "__rr_run_turn"]
            pub fn __rr_run() -> usize {
                with_mem(|mem| {
                    let input: $crate::ProgramInput = serde_json::from_slice(&mem).unwrap();
                    let output = CLOSURE.with(|c| {
                        let mut f = c.borrow_mut();
                        let f = f.as_mut().unwrap();
                        f(input)
                    });
                    mem.clear();
                    serde_json::to_writer(mem, &output).unwrap();
                })
            }
        };
    };
}
