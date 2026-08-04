use crate::command::CommandError;

pub(super) fn required_argument<'a, I>(
    parts: &mut I,
    usage: &'static str,
) -> Result<&'a str, CommandError>
where
    I: Iterator<Item = &'a str>,
{
    parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or(CommandError::InvalidArguments(usage))
}

pub(super) fn ensure_no_extra_arguments<'a, I>(
    parts: &mut I,
    usage: &'static str,
) -> Result<(), CommandError>
where
    I: Iterator<Item = &'a str>,
{
    if parts.next().is_some() {
        return Err(CommandError::InvalidArguments(usage));
    }

    Ok(())
}
