use crate::util::*;
use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Serialize)]
pub struct AlpacaResponse<T: AlpacaResponseValue> {
    #[serde(
        rename = "Value",
        skip_serializing_if = "should_skip_serializing",
        with = "alpaca_serializer"
    )]
    value: Option<T>,
    #[serde(rename = "ErrorNumber")]
    error_number: i32,
    #[serde(rename = "ErrorMessage")]
    error_message: String,
    #[serde(rename = "ClientTransactionID")]
    client_transaction_id: u32,
    #[serde(rename = "ServerTransactionID")]
    server_transaction_id: u32,
}

impl<T> AlpacaResponse<T>
where
    T: AlpacaResponseValue,
{
    pub fn new(result: AscomResult<T>, client_transaction_id: u32, sti: &AtomicU32) -> Self {
        let server_transaction_id = Self::generate_server_transaction_id(sti);
        match result {
            Ok(v) => Self {
                value: Some(v),
                error_number: 0,
                error_message: "".to_string(),
                client_transaction_id,
                server_transaction_id,
            },
            Err(e) => Self {
                value: None,
                error_number: e.error_number,
                error_message: e.error_message,
                client_transaction_id,
                server_transaction_id,
            },
        }
    }

    fn generate_server_transaction_id(sti: &AtomicU32) -> u32 {
        sti.fetch_add(1, Ordering::Relaxed)
    }
}

fn should_skip_serializing<T>(v: &Option<T>) -> bool
where
    T: AlpacaResponseValue,
{
    match v {
        Some(a) => a.skip(),
        None => true,
    }
}

mod alpaca_serializer {
    use super::AlpacaResponseValue;
    use serde::Serializer;

    pub fn serialize<T, S>(v: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AlpacaResponseValue,
        S: Serializer,
    {
        match v {
            Some(v) => v.serialize(serializer),
            None => unreachable!(),
        }
    }
}

pub trait AlpacaResponseValue {
    fn skip(&self) -> bool;
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

/* Workaround for overriding existing Serialize implementations */
pub trait AlpacaSerializeDefault {}

impl AlpacaSerializeDefault for bool {}
impl AlpacaSerializeDefault for u32 {}
impl AlpacaSerializeDefault for i32 {}
impl AlpacaSerializeDefault for f64 {}
impl AlpacaSerializeDefault for &str {}
impl AlpacaSerializeDefault for String {}
impl AlpacaSerializeDefault for &[&str] {}
impl AlpacaSerializeDefault for AlignmentMode {}
impl AlpacaSerializeDefault for TrackingRate {}
impl AlpacaSerializeDefault for Vec<TrackingRate> {}
impl AlpacaSerializeDefault for EquatorialCoordinateType {}
impl AlpacaSerializeDefault for PierSide {}
impl AlpacaSerializeDefault for Vec<AxisRateRange> {}

impl<T> AlpacaResponseValue for T
where
    T: Serialize + AlpacaSerializeDefault,
{
    fn skip(&self) -> bool {
        false
    }

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.serialize(serializer)
    }
}

impl AlpacaResponseValue for () {
    fn skip(&self) -> bool {
        true
    }

    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!()
    }
}

impl AlpacaResponseValue for DateTime<Utc> {
    fn skip(&self) -> bool {
        false
    }

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", self.format(ALPACA_DATE_FMT));
        serializer.serialize_str(&s)
    }
}
