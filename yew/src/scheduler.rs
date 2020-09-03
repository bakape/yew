//! This module contains a scheduler.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

// /// Prevents recursion by buffering calls and maintains separate buffers for synchronous jobs, jobs
// /// deferred to the next animation frame and debounced jobs.
// // TODO: Reuse this on yew::scheduler to reduce Rc and RefCell spam overhead and enable diffing +
// // rendering only on animation frames.
// #[derive(Default)]
// pub struct Scheduler<S, DF = S>
// where
//     S: Buffer,
//     DF: Buffer,
// {
//     /// Prevents job recursion
//     lock: RefCell<()>,

//     /// Buffer for jobs that can not be executed at once but should be executed as soon as possible.
//     synchronous: RefCell<S>,

//     /// Buffer for jobs to be executed on the next animation frame
//     // TODO: exec all sync jobs after this completes
//     // TODO: buffer and perform all component diffs+updates here (with deduplication), when this
//     // replaces the current scheduler.
//     deferred: RefCell<DF>,
//     //
//     // TODO: debounce using (EventDescriptor, listenerID). Figure out how to make this generic
//     // TODO: exec all sync jobs after this completes
//     // TODO: debounced calls should execute first because they can generate component updates
// }

// impl<S, DF> Scheduler<S, DF>
// where
//     S: Buffer,
//     DF: Buffer,
// {
//     /// Schedule job with priority p for ASAP execution
//     pub fn schedule(&self, p: S::Priority, job: impl FnOnce() + 'static) {
//         // Allow only one job to run at a time
//         match self.lock.try_borrow_mut() {
//             Ok(_) => {
//                 job();
//                 Self::run_buffered(&self.synchronous);
//             }
//             _ => {
//                 Self::buffer(&self.synchronous, p, job);
//             }
//         }
//     }

//     /// Schedule job with priority p to be executed on the next animation frame
//     pub fn schedule_deferred(&self, p: DF::Priority, job: impl FnOnce() + 'static) {
//         Self::buffer(&self.deferred, p, job);

//         // TODO: Schedule render task, if none
//     }

//     fn buffer<B>(dst: &RefCell<B>, p: B::Priority, job: impl FnOnce() + 'static)
//     where
//         B: Buffer,
//     {
//         dst.borrow_mut()
//             .buffer(p, Box::new(job) as Box<dyn FnOnce()>);
//     }

//     /// Run any buffered jobs from src
//     fn run_buffered<B>(src: &RefCell<B>)
//     where
//         B: Buffer,
//     {
//         while let Some(job) = src.borrow_mut().next() {
//             job();
//         }
//     }
// }

// /// Buffers incoming work and instructs Scheduler what to run next
// pub trait Buffer: Default {
//     /// Enables prioritizing some jobs over others
//     type Priority;

//     /// Buffer job with priority p for later execution
//     fn buffer(&mut self, p: Self::Priority, job: Box<dyn FnOnce()>);

//     /// Return next job to execute, if any
//     fn next(&mut self) -> Option<Box<dyn FnOnce()>>;
// }

// #[derive(Default)]
// struct FIFO(VecDeque<Box<dyn FnOnce()>>);

// impl Buffer for FIFO {
//     type Priority = ();

//     fn buffer(&mut self, _: Self::Priority, job: Box<dyn FnOnce()>) {
//         self.0.push_back(job);
//     }

//     fn next(&mut self) -> Option<Box<dyn FnOnce()>> {
//         self.0.pop_front()
//     }
// }

pub(crate) type Shared<T> = Rc<RefCell<T>>;

thread_local! {
    static SCHEDULER: Rc<Scheduler> =
        Rc::new(Scheduler::new());
}

pub(crate) fn scheduler() -> Rc<Scheduler> {
    SCHEDULER.with(Rc::clone)
}

/// A routine which could be run.
pub(crate) trait Runnable {
    /// Runs a routine with a context instance.
    fn run(self: Box<Self>);
}

/// This is a global scheduler suitable to schedule and run any tasks.
#[derive(Clone)]
pub(crate) struct Scheduler {
    /// This lock is used to prevent recursion in [Scheduler#start()](Scheduler#start())
    lock: Rc<RefCell<()>>,
    main: Shared<VecDeque<Box<dyn Runnable>>>,
    component: ComponentScheduler,
}

pub(crate) enum ComponentRunnableType {
    Destroy,
    Create,
    Update,
    Render,
    Rendered,
}

#[derive(Clone)]
struct ComponentScheduler {
    // Queues
    destroy: Shared<VecDeque<Box<dyn Runnable>>>,
    create: Shared<VecDeque<Box<dyn Runnable>>>,
    update: Shared<VecDeque<Box<dyn Runnable>>>,
    render: Shared<VecDeque<Box<dyn Runnable>>>,

    // Stack
    rendered: Shared<Vec<Box<dyn Runnable>>>,
}

impl ComponentScheduler {
    fn new() -> Self {
        ComponentScheduler {
            destroy: Rc::new(RefCell::new(VecDeque::new())),
            create: Rc::new(RefCell::new(VecDeque::new())),
            update: Rc::new(RefCell::new(VecDeque::new())),
            render: Rc::new(RefCell::new(VecDeque::new())),
            rendered: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn next_runnable(&self) -> Option<Box<dyn Runnable>> {
        None.or_else(|| self.destroy.borrow_mut().pop_front())
            .or_else(|| self.create.borrow_mut().pop_front())
            .or_else(|| self.update.borrow_mut().pop_front())
            .or_else(|| self.render.borrow_mut().pop_front())
            .or_else(|| self.rendered.borrow_mut().pop())
    }
}

impl Scheduler {
    fn new() -> Self {
        Scheduler {
            lock: Rc::new(RefCell::new(())),
            main: Rc::new(RefCell::new(VecDeque::new())),
            component: ComponentScheduler::new(),
        }
    }

    pub(crate) fn push_comp(&self, run_type: ComponentRunnableType, runnable: Box<dyn Runnable>) {
        match run_type {
            ComponentRunnableType::Destroy => {
                self.component.destroy.borrow_mut().push_back(runnable)
            }
            ComponentRunnableType::Create => self.component.create.borrow_mut().push_back(runnable),
            ComponentRunnableType::Update => self.component.update.borrow_mut().push_back(runnable),
            ComponentRunnableType::Render => self.component.render.borrow_mut().push_back(runnable),
            ComponentRunnableType::Rendered => self.component.rendered.borrow_mut().push(runnable),
        };
        self.start();
    }

    pub(crate) fn push_comp_update_batch(&self, it: impl IntoIterator<Item = Box<dyn Runnable>>) {
        self.component.update.borrow_mut().extend(it);
        self.start();
    }

    pub(crate) fn push(&self, runnable: Box<dyn Runnable>) {
        self.main.borrow_mut().push_back(runnable);
        self.start();
    }

    pub(crate) fn lock(&self) -> Option<std::cell::Ref<'_, ()>> {
        self.lock.try_borrow().ok()
    }

    fn next_runnable(&self) -> Option<Box<dyn Runnable>> {
        None.or_else(|| self.component.next_runnable())
            .or_else(|| self.main.borrow_mut().pop_front())
    }

    pub(crate) fn start(&self) {
        // The lock is used to prevent recursion. If the lock
        // cannot be acquired, it is because the `start()` method
        // is being called recursively as part of a `runnable.run()`.
        if let Ok(_lock) = self.lock.try_borrow_mut() {
            while let Some(runnable) = self.next_runnable() {
                runnable.run();
            }
        }
    }
}
