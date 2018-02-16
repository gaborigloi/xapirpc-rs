extern crate base64;
extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate xmlrpc;

use std::str::FromStr;

use base64::encode;
use clap::{Arg, App};
use xmlrpc::{Request, Value};
use reqwest::Client;
use serde_json::value as json;

fn to_value(value: &str) -> Value {
    bool::from_str(value)
        .map(|b| Value::Bool(b))
        .unwrap_or(
            i64::from_str(value)
            .map(|i| Value::Int64(i))
            .unwrap_or(
                f64::from_str(value)
                .map(|f| Value::Double(f))
                .unwrap_or(
                    Value::String(value.to_string())
                    )))
}


// The two functions below should be changed to return a Result
// instead of panicking

fn get_value(res: &Value) -> &Value {
    if let Value::Struct(ref response) = *res {
        &response["Value"]
    } else {
        panic!("Malformed response: {:?}", res)
    }
}

fn extract_session(res: Value) -> String {
    let value = get_value(&res);
    if let Value::String(ref session) = *value {
        session.clone()
    } else {
        panic!("Mismatched type: {:?}", value)
    }
}

fn as_json(value: &Value) -> json::Value {
    match *value {
        Value::Int(i) => {
            let i = json::Number::from_f64(i as f64).unwrap();
            json::Value::Number(i)
        }
        Value::Int64(i) => {
            let i = json::Number::from_f64(i as f64).unwrap();
            json::Value::Number(i)
        }
        Value::Bool(b) => {
            json::Value::Bool(b)
        }
        Value::String(ref s) => {
            json::Value::String(s.clone())
        }
        Value::Double(d) => {
            let d = json::Number::from_f64(d).unwrap();
            json::Value::Number(d)
        }
        Value::DateTime(date_time) => {
            json::Value::String(format!("{:?}", date_time))
        }
        Value::Base64(ref data) => {
            json::Value::String(encode(data))
        }
        Value::Struct(ref map) => {
            let mut jmap = serde_json::Map::with_capacity(map.len());
            for (ref name, ref value) in map {
                jmap.insert(
                    name.to_string().clone(),
                    as_json(value)
                    );
            };
            json::Value::Object(jmap)
        }
        Value::Array(ref array) => {
            json::Value::Array(
                array.iter().map(|v| as_json(v)).collect()
                )
        }
        Value::Nil => {
            json::Value::Null
        }
    }
}
fn main() {
    let matches = App::new("Dummy xapi xmlrpc CLI client")
        .version("0.1")
        .author("Marcello S. <marcello.seri@citrix.com>")
        .about("CLI interface to interrogate an instance of XenServer via xmlrpc")
        .arg(Arg::with_name("host")
             .long("host")
             .value_name("HOST")
             .env("XAPI_HOST")
             .help("XenServer host. Can be passed with the HOST env variable.")
             .takes_value(true))
        .arg(Arg::with_name("user")
             .short("u")
             .long("user")
             .value_name("USER")
             .env("XAPI_USER")
             .help("XenServer host user name. Can be passed with the \
                   XAPI_USER env variable.")
             .takes_value(true))
        .arg(Arg::with_name("pass")
             .short("p")
             .long("pass")
             .value_name("PASSWORD")
             .env("XAPI_PASSWORD")
             .help("XenServer host user password. Can be passed with the \
                   XAPI_PASSWORD env variable.")
             .takes_value(true))
        .arg(Arg::with_name("compact")
             .long("compact")
             .help("Output the result as non-prettified json.")
            )
        .arg(Arg::with_name("class")
             .value_name("CLASS")
             .help("Case sensitive value for the xapi class.")
             .required(true)
             .index(1))
        .arg(Arg::with_name("method")
             .value_name("METHOD")
             .help("Case sensitive value for the xapi method.")
             .required(true)
             .index(2))
        .arg(Arg::with_name("args")
             .value_name("ARGS")
             .help("Ordered list of arguments for the call (if any). \
                   Do not pass a session.")
             .multiple(true))
        .get_matches();

    let host = matches.value_of("host").unwrap_or("http://127.0.0.1");
    let user = matches.value_of("user").unwrap_or("guest");
    let pass = matches.value_of("pass").unwrap_or("guest");

    // These are compulsory parameters. unwrapping here is fine
    let class = matches.value_of("class").unwrap();
    let method = matches.value_of("method").unwrap();

    let args = if matches.is_present("args") {
        matches.values_of("args").unwrap().collect()
    } else {
        Vec::new()
    }.into_iter()
    .map(|a| to_value(&a));

    let client = Client::new();

    // Let's panic all the way!!!
    let hopefully_session =
        Request::new("session.login_with_password")
        .arg(user).arg(pass)
        .call(&client, host)
        .unwrap()  // Response
        .unwrap(); // Result
    let session = extract_session(hopefully_session);
    //println!("Session: \"{}\"", session);

    // Prepare the xmlrpc command
    let cmd = format!("{}.{}", class, method);
    let mut req =
        Request::new(&cmd)
        .arg(session.clone());
    for arg in args {
        req = req.arg(arg);
    }

    // Again, we unwrap the Response and then the Result
    // don't care about the panics right now.
    let response = req.call(&client, host).unwrap().unwrap();

    let json_value = as_json(get_value(&response));
    let j = if matches.is_present("compact") {
        serde_json::to_string(&json_value)
    } else {
        serde_json::to_string_pretty(&json_value)
    };
    println!("{}", j.unwrap());

    let _ =
        Request::new("session.logout")
        .arg(session)
        .call(&client, host);
}
