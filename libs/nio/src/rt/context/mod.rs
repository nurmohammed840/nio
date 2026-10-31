mod local;
mod runtime;

use std::{cell::UnsafeCell, rc::Rc};

use super::*;
use worker::WorkerId;

pub use local::LocalContext;
pub use runtime::RuntimeContext;

thread_local! {
    static CONTEXT: UnsafeCell<NioContext> = const { UnsafeCell::new(NioContext::None) };
}

#[derive(Clone)]
pub enum NioContext {
    None,
    Runtime(Arc<RuntimeContext>),
    Local(Rc<LocalContext>),
}

impl NioContext {
    #[inline(never)]
    fn panic_if_exist(&self) {
        match self {
            NioContext::Runtime(_) => panic!("global runtime already exist"),
            NioContext::Local(_) => panic!("local runtime already exist"),
            NioContext::None => {}
        }
    }

    #[inline(never)]
    fn panic_if_local_ctx_not_found(&self) {
        match self {
            NioContext::Runtime(_) => panic!("expected local runtime, found global runtime"),
            NioContext::None => panic!("no local runtime exist"),
            NioContext::Local(_) => {}
        }
    }

    fn init(local: Rc<LocalContext>) {
        CONTEXT.with(|ctx| unsafe {
            (*ctx.get()).panic_if_exist();
            *ctx.get() = NioContext::Local(local)
        });
    }

    pub fn drop_local_context() {
        CONTEXT.with(|ctx| unsafe {
            (*ctx.get()).panic_if_local_ctx_not_found();
            *ctx.get() = NioContext::None
        });
    }

    fn enter(rt: Arc<RuntimeContext>) {
        CONTEXT.with(|ctx| unsafe {
            (*ctx.get()).panic_if_exist();
            *ctx.get() = NioContext::Runtime(rt)
        });
    }

    pub fn get<F, R>(f: F) -> R
    where
        F: FnOnce(&NioContext) -> R,
    {
        CONTEXT.with(|ctx| unsafe { f(&*ctx.get()) })
    }
}

#[inline(never)]
pub fn no_rt_found_panic() -> ! {
    panic!("no `Nio` runtime available");
}

#[inline(never)]
pub fn no_local_rt_found_panic() -> ! {
    panic!("no `Nio` local runtime found");
}
