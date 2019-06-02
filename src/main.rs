use failure::Fail;
use futures::Future;
use rusoto_core::{credential::ChainProvider, request::HttpClient, RusotoError};
use rusoto_lambda::{
    Environment, FunctionConfiguration, GetFunctionConfigurationError,
    GetFunctionConfigurationRequest, Lambda, LambdaClient, UpdateFunctionConfigurationRequest,
};
use std::{
    collections::HashMap, error::Error as StdError, process::exit, str::FromStr,
    time::Duration,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

// Ours
mod error;
use crate::error::Error;

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<std::error::Error>>
where
    T: FromStr,
    T::Err: StdError + 'static,
    U: FromStr,
    U::Err: StdError + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
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

fn get<F>(
    lambda: LambdaClient,
    function: F,
) -> impl Future<Item = Env, Error = RusotoError<GetFunctionConfigurationError>> + Send
where
    F: Into<String>,
{
    lambda
        .get_function_configuration(GetFunctionConfigurationRequest {
            function_name: function.into(),
            ..GetFunctionConfigurationRequest::default()
        })
        .map(env)
}

fn set<F>(
    lambda: LambdaClient,
    function: F,
    vars: Vec<(String, String)>,
) -> impl Future<Item = Env, Error = Error> + Send
where
    F: Into<String>,
{
    let function = function.into();
    get(lambda.clone(), function.clone())
        .map_err(Error::from)
        .and_then(move |current| {
            let updated = current.into_iter().chain(vars).collect();
            lambda
                .update_function_configuration(UpdateFunctionConfigurationRequest {
                    function_name: function,
                    environment: Some(Environment {
                        variables: Some(updated),
                    }),
                    ..UpdateFunctionConfigurationRequest::default()
                })
                .map(env)
                .map_err(Error::from)
        })
}

fn unset<F>(
    lambda: LambdaClient,
    function: F,
    names: Vec<String>,
) -> impl Future<Item = Env, Error = Error> + Send
where
    F: Into<String>,
{
    let function = function.into();
    get(lambda.clone(), function.clone())
        .map_err(Error::from)
        .and_then(move |current| {
            let updated = current
                .into_iter()
                .filter(|(k, _)| !names.contains(k))
                .collect();
            lambda
                .update_function_configuration(UpdateFunctionConfigurationRequest {
                    function_name: function,
                    environment: Some(Environment {
                        variables: Some(updated),
                    }),
                    ..UpdateFunctionConfigurationRequest::default()
                })
                .map(env)
                .map_err(Error::from)
        })
}

fn render(env: Env) {
    for (k, v) in env {
        println!("{}={}", k, v)
    }
}

fn credentials() -> ChainProvider {
    let mut chain = ChainProvider::new();
    chain.set_timeout(Duration::from_millis(200));
    chain
}

fn lambda_client() -> LambdaClient {
    LambdaClient::new_with(
        HttpClient::new().expect("failed to create request dispatcher"),
        credentials(),
        Default::default(),
    )
}

fn main() {
    let mut rt = Runtime::new().expect("failed to initialize runtime");
    let result = match Options::from_args() {
        Options::Get { function } => rt.block_on(
            get(lambda_client(), function)
                .map_err(Error::from)
                .map(render),
        ),
        Options::Set { function, vars } => rt.block_on(
            set(lambda_client(), function, vars)
                .map_err(Error::from)
                .map(render),
        ),
        Options::Unset { function, names } => rt.block_on(
            unset(lambda_client(), function, names)
                .map_err(Error::from)
                .map(render),
        ),
    };
    if let Err(err) = result {
        for cause in Fail::iter_causes(&err) {
            eprintln!("{}", cause);
        }
        exit(1)
    }
}

#[cfg(test)]
mod tests {
    use super::{env, Options};
    use rusoto_lambda::{EnvironmentResponse, FunctionConfiguration};
    use std::collections::HashMap;

    use structopt::StructOpt;

    #[test]
    fn env_extracts_from_empty_config() {
        assert_eq!(
            env(FunctionConfiguration {
                ..FunctionConfiguration::default()
            }),
            Default::default()
        )
    }

    #[test]
    fn env_extracts_from_nonempty_config() {
        let mut vars = HashMap::new();
        vars.insert("foo".to_string(), "bar".to_string());
        assert_eq!(
            env(FunctionConfiguration {
                environment: Some(EnvironmentResponse {
                    variables: Some(vars.clone()),
                    ..EnvironmentResponse::default()
                }),
                ..FunctionConfiguration::default()
            }),
            vars
        )
    }

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
