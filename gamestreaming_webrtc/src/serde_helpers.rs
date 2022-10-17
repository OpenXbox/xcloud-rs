/// Helper to deserialize nested JSON
/// Reference: https://github.com/serde-rs/serde/issues/994#issuecomment-316895712
pub mod json_string {
    use serde::de::{self, Deserialize, DeserializeOwned, Deserializer};
    use serde::ser::{self, Serialize, Serializer};
    use serde_json;

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let j = serde_json::to_string(value).map_err(ser::Error::custom)?;
        j.serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        let j = String::deserialize(deserializer)?;
        serde_json::from_str(&j).map_err(de::Error::custom)
    }
}

/// RTCIceCandidateInit serde deserializer in webrtc-crate expects a non-normalized
/// representation of ICE json body
/// Expected (sdpMid: int as string, sdpMLineIndex: int):
/// ```
/// let _ = serde_json::json!({
///    "candidate":"a=candidate:1 1 UDP 100 43.111.100.34 1136 typ host ",
///    "sdpMid":"0",
///    "sdpMLineIndex":0,
///    "usernameFragment":null
/// });
/// ```
///
/// What is received back from XCloud HTTP API is the following:
/// (both, sdpMid and sdpMLineIndex, are ints as string)
/// ```
/// let _ = serde_json::json!({
///    "candidate":"a=candidate:1 1 UDP 100 43.111.100.34 1136 typ host ",
///    "sdpMid":"0",
///    "sdpMLineIndex":"0",
///    "usernameFragment":null
/// });
/// ```
///
/// FIXME: Remove this workaround, handle it in some better way
pub mod json_string_ice_workaround {
    use serde::Deserialize;
    use serde::de::{self, DeserializeOwned, Deserializer};
    use serde_json::{self, Value, json, Map};

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        fn deserialize_str_into_num(val: Value) -> Value {
            match val {
                Value::String(str) => {
                    if let Ok(num) = str.parse::<u8>() { json!(num) }
                    else if let Ok(num) = str.parse::<i8>() { json!(num) }
                    else if let Ok(num) = str.parse::<u16>() { json!(num) }
                    else if let Ok(num) = str.parse::<i16>() { json!(num) }
                    else if let Ok(num) = str.parse::<u32>() { json!(num) }
                    else if let Ok(num) = str.parse::<i32>() { json!(num) }
                    else if let Ok(num) = str.parse::<u64>() { json!(num) }
                    else if let Ok(num) = str.parse::<i64>() { json!(num) }
                    else if let Ok(num) = str.parse::<u128>() { json!(num) }
                    else if let Ok(num) = str.parse::<i128>() { json!(num) }
                    else { Value::String(str) }
                },
                _ => panic!("Expecting Value::String")
            }
        }

        fn deserialize_recursive(val: Value) -> Value {
            match val {
                Value::String(str) => deserialize_str_into_num(Value::String(str)),
                Value::Array(arr) => {
                    arr.into_iter().map(|val| {
                        deserialize_recursive(val)
                    }).collect()
                },
                Value::Object(obj) => {
                    let res = obj.into_iter().map(|(key,val)|{
                        if key == "sdpMLineIndex" {
                            return (key, deserialize_recursive(val));
                        }
                        (key, val)
                    }).collect::<Map<String, Value>>();
                    Value::Object(res)
                },
                v => v
            }
        }

        let j = String::deserialize(deserializer)?;
        let parsed = {
            if let Ok(val) = serde_json::from_str::<Vec<Value>>(&j) {
                Value::Array(val)
            } else if let Ok(val) = serde_json::from_str::<Map<String, Value>>(&j) {
                Value::Object(val)
            } else {
                return Err(de::Error::custom("Input is neither Array nor Object"));
            }
        };

        let raw = deserialize_recursive(parsed);
        serde_json::from_value(raw).map_err(de::Error::custom)
    }
}