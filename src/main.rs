#[macro_use]
extern crate structopt;
extern crate rusoto_lambda;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate rusoto_core;
extern crate tokio;

// Std
use std::collections::HashMap;
use std::error::Error as StdError;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

// Third party
use failure::Error;
use futures::Future;
use rusoto_core::credential::ChainProvider;
use rusoto_core::request::HttpClient;
use rusoto_lambda::{
    Environment, FunctionConfiguration, GetFunctionConfigurationError,
    GetFunctionConfigurationRequest, Lambda, LambdaClient, UpdateFunctionConfigurationError,
    UpdateFunctionConfigurationRequest,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

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

#[derive(Debug, Fail)]
enum LambdaError {
    #[fail(display = "{}", _0)]
    GetConfig(#[cause] GetFunctionConfigurationError),
    #[fail(display = "{}", _0)]
    UpdateConfig(#[cause] UpdateFunctionConfigurationError),
}

impl From<GetFunctionConfigurationError> for LambdaError {
    fn from(err: GetFunctionConfigurationError) -> Self {
        LambdaError::GetConfig(err)
    }
}

impl From<UpdateFunctionConfigurationError> for LambdaError {
    fn from(err: UpdateFunctionConfigurationError) -> Self {
        LambdaError::UpdateConfig(err)
    }
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

fn get<L, F>(
    lambda: Arc<L>,
    function: F,
) -> impl Future<Item = Env, Error = GetFunctionConfigurationError> + Send
where
    L: Lambda + Send,
    F: Into<String>,
{
    lambda
        .get_function_configuration(GetFunctionConfigurationRequest {
            function_name: function.into(),
            ..Default::default()
        })
        .map(env)
}

fn set<L, F>(
    lambda: Arc<L>,
    function: F,
    vars: Vec<(String, String)>,
) -> impl Future<Item = Env, Error = LambdaError> + Send
where
    L: Lambda + Send + Sync,
    F: Into<String>,
{
    let function = function.into();
    get(lambda.clone(), function.clone())
        .map_err(LambdaError::from)
        .and_then(move |current| {
            let updated = current.into_iter().chain(vars).collect();
            lambda
                .update_function_configuration(UpdateFunctionConfigurationRequest {
                    function_name: function,
                    environment: Some(Environment {
                        variables: Some(updated),
                    }),
                    ..Default::default()
                })
                .map(env)
                .map_err(LambdaError::from)
        })
}

fn unset<L, F>(
    lambda: Arc<L>,
    function: F,
    names: Vec<String>,
) -> impl Future<Item = Env, Error = LambdaError> + Send
where
    L: Lambda + Send + Sync,
    F: Into<String>,
{
    let function = function.into();
    get(lambda.clone(), function.clone())
        .map_err(LambdaError::from)
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
                    ..Default::default()
                })
                .map(env)
                .map_err(LambdaError::from)
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

fn main() -> Result<(), Error> {
    let mut rt = Runtime::new().expect("failed to initialize runtime");
    match Options::from_args() {
        Options::Get { function } => rt.block_on(
            get(Arc::new(lambda_client()), function)
                .map_err(Error::from)
                .map(render),
        ),
        Options::Set { function, vars } => rt.block_on(
            set(Arc::new(lambda_client()), function, vars)
                .map_err(Error::from)
                .map(render),
        ),
        Options::Unset { function, names } => rt.block_on(
            unset(Arc::new(lambda_client()), function, names)
                .map_err(Error::from)
                .map(render),
        ),
    }
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
