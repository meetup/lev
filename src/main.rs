#[macro_use]
extern crate structopt;
extern crate rusoto_lambda;
#[macro_use]
extern crate failure;

// Std
use std::collections::HashMap;
use std::error::Error as StdError;
use std::str::FromStr;

// Third party
use failure::Error;
use rusoto_lambda::{
    Environment, FunctionConfiguration, GetFunctionConfigurationError,
    GetFunctionConfigurationRequest, Lambda, LambdaClient, UpdateFunctionConfigurationRequest,
};
use structopt::StructOpt;

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<std::error::Error>>
where
    T: FromStr,
    T::Err: StdError + 'static,
    U: FromStr,
    U::Err: StdError + 'static,
{
    let pos = s.find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[derive(Debug, Fail)]
enum LambdaError {
    #[fail(display = "failed to get function config")]
    GetConfig,
    #[fail(display = "failed to update function config")]
    UpdateConfig,
}

#[derive(StructOpt, PartialEq, Debug)]
#[structopt(name = "lev", about = "AWS lambda env manager")]
enum Options {
    #[structopt(name = "get", about = "Gets a function's current env")]
    Get {
        #[structopt(short = "f", long = "function")]
        function: String,
    },
    #[structopt(name = "set", about = "Sets a function's env var")]
    Set {
        #[structopt(short = "f", long = "function")]
        function: String,
        #[structopt(name = "name=value", parse(try_from_str = "parse_key_val"))]
        vars: Vec<(String, String)>,
    },
    #[structopt(name = "unset", about = "Unsets a function's env var")]
    Unset {
        #[structopt(short = "f", long = "function")]
        function: String,
        #[structopt(name = "names")]
        names: Vec<String>,
    },
}

type Env = HashMap<String, String>;

fn env(conf: FunctionConfiguration) -> Env {
    conf.environment
        .map(|env| env.variables.unwrap_or_default())
        .unwrap_or_default()
}

fn get<F>(lambda: &Lambda, function: F) -> Result<Env, GetFunctionConfigurationError>
where
    F: Into<String>,
{
    lambda
        .get_function_configuration(&GetFunctionConfigurationRequest {
            function_name: function.into(),
            ..Default::default()
        })
        .sync()
        .map(env)
}

fn set<F>(lambda: &Lambda, function: F, vars: Vec<(String, String)>) -> Result<Env, LambdaError>
where
    F: Into<String>,
{
    let function = function.into();
    get(lambda, function.as_str())
        .map_err(|_| LambdaError::GetConfig)
        .and_then(|current| {
            let updated = current.into_iter().chain(vars).collect();
            lambda
                .update_function_configuration(&UpdateFunctionConfigurationRequest {
                    function_name: function,
                    environment: Some(Environment {
                        variables: Some(updated),
                    }),
                    ..Default::default()
                })
                .sync()
                .map(env)
                .map_err(|_| LambdaError::UpdateConfig)
        })
}

fn unset<F>(lambda: &Lambda, function: F, names: Vec<String>) -> Result<Env, LambdaError>
where
    F: Into<String>,
{
    let function = function.into();
    get(lambda, function.as_str())
        .map_err(|_| LambdaError::GetConfig)
        .and_then(|current| {
            let updated = current
                .into_iter()
                .filter(|(k, _)| !names.contains(k))
                .collect();
            lambda
                .update_function_configuration(&UpdateFunctionConfigurationRequest {
                    function_name: function,
                    environment: Some(Environment {
                        variables: Some(updated),
                    }),
                    ..Default::default()
                })
                .sync()
                .map(env)
                .map_err(|_| LambdaError::UpdateConfig)
        })
}

fn print(env: Env) {
    for (k, v) in env {
        println!("{}={}", k, v)
    }
}

fn main() -> Result<(), Error> {
    match Options::from_args() {
        Options::Get { function } => {
            print(get(&LambdaClient::simple(Default::default()), function)?)
        }
        Options::Set { function, vars } => print(set(
            &LambdaClient::simple(Default::default()),
            function,
            vars,
        )?),
        Options::Unset { function, names } => print(unset(
            &LambdaClient::simple(Default::default()),
            function,
            names,
        )?),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Options;
    use structopt::StructOpt;
    #[test]
    fn get_options() {
        assert_eq!(
            Options::Get {
                function: "foo".into()
            },
            Options::from_iter(&["lev", "get", "-f", "foo"])
        )
    }

    #[test]
    fn set_options() {
        assert_eq!(
            Options::Set {
                function: "foo".into(),
                vars: vec![("bar".into(), "baz".into()), ("boom".into(), "zoom".into())],
            },
            Options::from_iter(&["lev", "set", "-f", "foo", "bar=baz", "boom=zoom"])
        )
    }

    #[test]
    fn unset_options() {
        assert_eq!(
            Options::Unset {
                function: "foo".into(),
                names: vec!["bar".into(), "baz".into()],
            },
            Options::from_iter(&["lev", "unset", "-f", "foo", "bar", "baz"])
        )
    }
}
