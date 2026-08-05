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

pub(super) fn parse_key_value_command<'a>(
    input: &'a str,
    usage: &'static str,
) -> Result<(&'a str, &'a str), CommandError> {
    let mut parts = input.splitn(3, char::is_whitespace);

    parts.next();

    let key = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CommandError::InvalidArguments(usage))?;

    let value = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CommandError::InvalidArguments(usage))?;

    Ok((key, value))
}

pub(super) fn parse_integer_argument_command(
    input: &str,
    usage: &'static str,
) -> Result<(String, i64), CommandError> {
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    let amount = required_argument(&mut parts, usage)?;

    ensure_no_extra_arguments(&mut parts, usage)?;

    match amount.parse::<i64>() {
        Ok(amount) => Ok((key.to_owned(), amount)),
        Err(_) => Err(CommandError::InvalidInteger(amount.to_owned())),
    }
}
