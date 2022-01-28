use mlua::prelude::LuaValue;

use crate::value::Value;

impl<'a> ToLua<'a> for Value {
    #![allow(clippy::wrong_self_convention)] // this trait is defined by mlua
    fn to_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'_>> {
        match self {
            Value::Bytes(b) => lua.create_string(b.as_ref()).map(LuaValue::String),
            Value::Integer(i) => Ok(LuaValue::Integer(i)),
            Value::Float(f) => Ok(LuaValue::Number(f)),
            Value::Boolean(b) => Ok(LuaValue::Boolean(b)),
            Value::Timestamp(t) => timestamp_to_table(lua, t).map(LuaValue::Table),
            Value::Map(m) => lua.create_table_from(m.into_iter()).map(LuaValue::Table),
            Value::Array(a) => lua.create_sequence_from(a.into_iter()).map(LuaValue::Table),
            Value::Null => lua.create_string("").map(LuaValue::String),
        }
    }
}

impl<'a> FromLua<'a> for Value {
    fn from_lua(value: LuaValue<'a>, lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::String(s) => Ok(Value::Bytes(Vec::from(s.as_bytes()).into())),
            LuaValue::Integer(i) => Ok(Value::Integer(i)),
            LuaValue::Number(f) => Ok(Value::Float(f)),
            LuaValue::Boolean(b) => Ok(Value::Boolean(b)),
            LuaValue::Table(t) => {
                if t.len()? > 0 {
                    <_>::from_lua(LuaValue::Table(t), lua).map(Value::Array)
                } else if table_is_timestamp(&t)? {
                    table_to_timestamp(t).map(Value::Timestamp)
                } else {
                    <_>::from_lua(LuaValue::Table(t), lua).map(Value::Map)
                }
            }
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "Value",
                message: Some("Unsupported Lua type".to_string()),
            }),
        }
    }
}

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use mlua::prelude::*;

/// TODO: This was copy/pasted from Vector

/// Convert a `DateTime<Utc>` to a `LuaTable`
///
/// # Errors
///
/// This function will fail insertion into the table fails.
pub fn timestamp_to_table(lua: &Lua, ts: DateTime<Utc>) -> LuaResult<LuaTable<'_>> {
    let table = lua.create_table()?;
    table.raw_set("year", ts.year())?;
    table.raw_set("month", ts.month())?;
    table.raw_set("day", ts.day())?;
    table.raw_set("hour", ts.hour())?;
    table.raw_set("min", ts.minute())?;
    table.raw_set("sec", ts.second())?;
    table.raw_set("nanosec", ts.nanosecond())?;
    table.raw_set("yday", ts.ordinal())?;
    table.raw_set("wday", ts.weekday().number_from_sunday())?;
    table.raw_set("isdst", false)?;

    Ok(table)
}

/// Determines if a `LuaTable` is a timestamp.
///
/// # Errors
///
/// This function will fail if the table is malformed.
pub fn table_is_timestamp(t: &LuaTable<'_>) -> LuaResult<bool> {
    for &key in &["year", "month", "day", "hour", "min", "sec"] {
        if !t.contains_key(key)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Convert a `LuaTable` to a `DateTime<Utc>`
///
/// # Errors
///
/// This function will fail if the table is malformed.
#[allow(clippy::needless_pass_by_value)] // constrained by mlua types
pub fn table_to_timestamp(t: LuaTable<'_>) -> LuaResult<DateTime<Utc>> {
    let year = t.raw_get("year")?;
    let month = t.raw_get("month")?;
    let day = t.raw_get("day")?;
    let hour = t.raw_get("hour")?;
    let min = t.raw_get("min")?;
    let sec = t.raw_get("sec")?;
    let nano = t.raw_get::<_, Option<u32>>("nanosec")?.unwrap_or(0);
    Ok(Utc.ymd(year, month, day).and_hms_nano(hour, min, sec, nano))
}
