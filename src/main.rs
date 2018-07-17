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
    Environment, GetFunctionConfigurationError, GetFunctionConfigurationRequest, Lambda,
    LambdaClient, UpdateFunctionConfigurationRequest,
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
enum LamdaError {
    #[fail(display = "failed to get function config")]
    GetConfig,
    #[fail(display = "failed to update function config")]
    UpdateConfig,
}

#[derive(StructOpt)]
#[structopt(name = "lev", about = "AWS lambda env manager")]
enum App {
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

fn env<F>(
    lambda: &Lambda,
    function: F,
) -> Result<HashMap<String, String>, GetFunctionConfigurationError>
where
    F: Into<String>,
{
    lambda
        .get_function_configuration(&GetFunctionConfigurationRequest {
            function_name: function.into(),
            ..Default::default()
        })
        .sync()
        .map(|response| {
            response
                .environment
                .map(|env| env.variables.unwrap_or_default())
                .unwrap_or_default()
        })
}

fn set<F>(
    lambda: &Lambda,
    function: F,
    vars: Vec<(String, String)>,
) -> Result<HashMap<String, String>, LamdaError>
where
    F: Into<String>,
{
    let function = function.into();
    env(lambda, function.as_str())
        .map_err(|_| LamdaError::GetConfig)
        .and_then(|env| {
            let updated = env.into_iter().chain(vars).collect();
            lambda
                .update_function_configuration(&UpdateFunctionConfigurationRequest {
                    function_name: function,
                    environment: Some(Environment {
                        variables: Some(updated),
                    }),
                    ..Default::default()
                })
                .sync()
                .map_err(|_| LamdaError::UpdateConfig)
                .map(|response| {
                    response
                        .environment
                        .map(|env| env.variables.unwrap_or_default())
                        .unwrap_or_default()
                })
        })
}

fn unset<F>(
    lambda: &Lambda,
    function: F,
    names: Vec<String>,
) -> Result<HashMap<String, String>, LamdaError>
where
    F: Into<String>,
{
    let function = function.into();
    env(lambda, function.as_str())
        .map_err(|_| LamdaError::GetConfig)
        .and_then(|env| {
            let updated = env.into_iter()
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
                .map_err(|_| LamdaError::UpdateConfig)
                .map(|response| {
                    response
                        .environment
                        .map(|env| env.variables.unwrap_or_default())
                        .unwrap_or_default()
                })
        })
}

fn print(env: HashMap<String, String>) {
    for (k, v) in env {
        println!("{}={}", k, v)
    }
}

fn main() -> Result<(), Error> {
    match App::from_args() {
        App::Get { function } => print(env(&LambdaClient::simple(Default::default()), function)?),
        App::Set { function, vars } => print(set(
            &LambdaClient::simple(Default::default()),
            function,
            vars,
        )?),
        App::Unset { function, names } => print(unset(
            &LambdaClient::simple(Default::default()),
            function,
            names,
        )?),
    }
    Ok(())
}
