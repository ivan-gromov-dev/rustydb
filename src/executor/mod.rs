mod execute;

pub(crate) use execute::execute_with_snapshot;

#[cfg(test)]
pub(crate) use execute::execute;

#[cfg(test)]
mod tests;
