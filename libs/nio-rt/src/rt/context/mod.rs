mod local;
mod runtime;

use std::{cell::UnsafeCell, rc::Rc};

use super::*;
use worker::WorkerId;

pub use local::LocalContext;
pub use runtime::RuntimeContext;

thread_local! {
    static CONTEXT: UnsafeCell<Context> = const { UnsafeCell::new(Context::None) };
}

#[derive(Clone)]
pub enum Context {
    None,
    Global(Arc<RuntimeContext>),
    Local(Rc<LocalContext>),
}

impl Context {
    fn panic_if_exist(&self) {
        match self {
            Context::Global(_) => panic!("global runtime already exist"),
            Context::Local(_) => panic!("local runtime already exist"),
            Context::None => {}
        }
    }

    fn init(local: Rc<LocalContext>) {
        CONTEXT.with(|ctx| unsafe {
            (*ctx.get()).panic_if_exist();
            *ctx.get() = Context::Local(local)
        });
    }

    fn enter(rt: Arc<RuntimeContext>) {
        CONTEXT.with(|ctx| unsafe {
            (*ctx.get()).panic_if_exist();
            *ctx.get() = Context::Global(rt)
        });
    }

    pub fn get<F, R>(f: F) -> R
    where
        F: FnOnce(&Context) -> R,
    {
        CONTEXT.with(|ctx| unsafe { f(&*ctx.get()) })
    }
}
