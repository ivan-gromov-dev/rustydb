mod listener;

pub use listener::run_server;

#[cfg(test)]
pub(crate) use listener::serve;

#[cfg(test)]
mod tests;
