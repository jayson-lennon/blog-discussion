#![allow(warnings)]
use std::{any::Any, cell::RefCell, collections::VecDeque, marker::PhantomData, rc::Rc};

use derive_more::Debug;

pub type AnyMsg = Box<dyn Any>;

#[derive(Default)]
pub struct Ctx {
    msgs: Vec<AnyMsg>,
}

impl Ctx {
    pub fn send<M>(&mut self, msg: M)
    where
        M: 'static,
    {
        self.msgs.push(Box::new(msg));
    }
}

pub trait Observer<M> {
    fn handle(&mut self, ctx: &mut Ctx, msg: &M);
}

pub type UserId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLoggedIn(pub UserId);

pub struct UpdateLastLoginTime;

impl Observer<UserLoggedIn> for UpdateLastLoginTime {
    fn handle(&mut self, ctx: &mut Ctx, _msg: &UserLoggedIn) {
        // (update a database with the login time)
    }
}

// Let's make our message type while we're here in a code block...
pub struct SayHello(pub String);
pub struct SayGoodbye(pub String);

#[derive(Debug, Default)]
pub struct GreetingObserver {
    greet_count: u32,
}

impl Observer<SayHello> for GreetingObserver {
    fn handle(&mut self, ctx: &mut Ctx, hello: &SayHello) {
        self.greet_count += 1;
        println!("Hello, {}!", hello.0);
        ctx.send(SayGoodbye("blog".to_owned()));
    }
}

impl Observer<SayGoodbye> for GreetingObserver {
    fn handle(&mut self, ctx: &mut Ctx, goodbye: &SayGoodbye) {
        self.greet_count += 1;
        println!("Goodbye, {}!", goodbye.0);
    }
}

pub struct MessageSystem {
    observers: Vec<Box<dyn ErasedObserver>>,
}

impl MessageSystem {
    pub fn add_observer<T, M>(&mut self, observer: Rc<RefCell<T>>)
    where
        T: Observer<M> + 'static,
        M: 'static,
    {
        let observer = Box::new(ObserverWrapper::new(observer));
        self.observers.push(observer);
    }

    pub fn send<M>(&self, msg: M)
    where
        M: 'static,
    {
        let mut msgs: VecDeque<AnyMsg> = VecDeque::new();
        msgs.push_back(Box::new(msg));
        while let Some(msg) = msgs.pop_front() {
            for observer in &self.observers {
                observer.handle_any(&mut msgs, &*msg);
            }
        }
    }
}

impl Default for MessageSystem {
    fn default() -> Self {
        Self {
            observers: Default::default(),
        }
    }
}

trait ErasedObserver {
    fn handle_any(&self, send_msgs: &mut VecDeque<AnyMsg>, msg: &dyn Any);
}

struct ObserverWrapper<T, M>
where
    T: Observer<M>,
{
    observer: Rc<RefCell<T>>,
    _phantom: PhantomData<M>,
}

impl<T, M> ObserverWrapper<T, M>
where
    T: Observer<M>,
{
    pub fn new(observer: Rc<RefCell<T>>) -> Self {
        Self {
            observer,
            _phantom: PhantomData,
        }
    }
}

impl<T, M> ErasedObserver for ObserverWrapper<T, M>
where
    T: Observer<M>,
    M: 'static,
{
    fn handle_any(&self, send_msgs: &mut VecDeque<AnyMsg>, msg: &dyn Any) {
        if let Some(msg) = msg.downcast_ref::<M>() {
            let mut ctx = Ctx::default();
            let mut observer = self.observer.borrow_mut();
            observer.handle(&mut ctx, msg);
            send_msgs.extend(ctx.msgs.into_iter());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_multiple_message_types() {
        let mut system = MessageSystem::default();
        let observer = Rc::new(RefCell::new(GreetingObserver::default()));
        system.add_observer::<_, SayHello>(observer.clone());
        system.add_observer::<_, SayGoodbye>(observer.clone());

        let msg = SayHello("world".to_string());
        system.send(msg);

        let msg_count = observer.borrow().greet_count;
        assert_eq!(msg_count, 2);
    }
}
