use crate::{api, api::IsParser};
use failure::Fail;
use wasm_bindgen::prelude::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "JSON (de)serialization failed: {:?}", _0)]
    JsonSerializationError(#[cause] serde_json::error::Error)
}

impl From<Error> for api::Error {
    fn from(e: Error) -> Self {
        api::interop_error(e)
    }
}
impl From<serde_json::error::Error> for Error {
    fn from(error: serde_json::error::Error) -> Self {
        Error::JsonSerializationError(error)
    }
}


/// Wrapper over the JS-compiled parser.
///
/// Can only be used when targeting WebAssembly.
pub struct Client {}

impl Client {
    #[allow(dead_code)]
    pub fn new() -> Result<Client> {
       let path = std::env::current_dir()?;
       let jsparser = std
        Ok(Client {})
    }
}

impl IsParser for Client {
    fn parse(&mut self, _program: String) -> api::Result<api::AST> {
        Ok(parse(&_program))
    }
}


#[wasm_bindgen(module = "/foo.js")]
extern "C" {
   fn parse(program: &str) -> String;
}