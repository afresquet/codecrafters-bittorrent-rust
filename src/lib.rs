use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum Bencode {
    String(String),
    Number(isize),
    List(Vec<Bencode>),
    Dictionary(HashMap<String, Bencode>),
}

impl Bencode {
    pub fn new(encoded_value: &str) -> Result<Self, BencodeError> {
        match encoded_value.chars().next() {
            Some('0'..='9') => {
                let (number_string, string) = encoded_value
                    .split_once(':')
                    .ok_or(BencodeError::MissingDelimeter)?;
                let number = number_string
                    .parse()
                    .map_err(|_| BencodeError::InvalidNumber)?;
                let decoded_value = string.get(..number).ok_or(BencodeError::InvalidLength)?;
                Ok(Self::String(decoded_value.to_string()))
            }
            Some('i') => {
                let end = encoded_value
                    .find('e')
                    .ok_or(BencodeError::MissingDelimeter)?;
                let number = encoded_value
                    .get(1..end)
                    .expect("is within bounds")
                    .parse()
                    .map_err(|_| BencodeError::InvalidNumber)?;
                Ok(Self::Number(number))
            }
            Some('l') => {
                let mut decoded_values = Vec::new();
                let mut rest = &encoded_value[1..];

                loop {
                    match rest.chars().next() {
                        Some('e') => break,
                        None => return Err(BencodeError::MissingDelimeter),
                        _ => {
                            let value = Self::new(rest)?;
                            rest = &rest[value.encoded_length()..];
                            decoded_values.push(value);
                        }
                    }
                }

                Ok(Self::List(decoded_values))
            }
            Some('d') => {
                let mut decoded_values = HashMap::new();
                let mut rest = &encoded_value[1..];

                loop {
                    match rest.chars().next() {
                        Some('e') => break,
                        None => return Err(BencodeError::MissingDelimeter),
                        _ => {
                            let key = Self::new(rest)?;
                            rest = &rest[key.encoded_length()..];
                            let Self::String(key) = key else {
                                return Err(BencodeError::InvalidKey);
                            };
                            let value = Self::new(rest)?;
                            rest = &rest[value.encoded_length()..];
                            decoded_values.insert(key, value);
                        }
                    }
                }

                Ok(Self::Dictionary(decoded_values))
            }
            Some(c) => unreachable!("Invalid delimeter {c}"),
            None => Err(BencodeError::EmptyInput),
        }
    }

    fn encoded_length(&self) -> usize {
        match self {
            Bencode::String(s) => s.len() + s.len().to_string().len() + 1,
            Bencode::Number(n) => n.to_string().len() + 2,
            Bencode::List(l) => l.iter().map(|v| v.encoded_length()).sum::<usize>() + 2,
            Bencode::Dictionary(d) => {
                d.iter()
                    .map(|(k, v)| k.len() + 2 + v.encoded_length())
                    .sum::<usize>()
                    + 2
            }
        }
    }
}

impl From<&Bencode> for Value {
    fn from(value: &Bencode) -> Self {
        match value {
            Bencode::String(string) => Value::String(string.to_owned()),
            Bencode::Number(number) => Value::Number((*number).into()),
            Bencode::List(values) => Value::Array(values.iter().map(Into::into).collect()),
            Bencode::Dictionary(dictionary) => Value::Object(
                dictionary
                    .iter()
                    .map(|(k, v)| (k.to_owned(), v.into()))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BencodeError {
    EmptyInput,
    MissingDelimeter,
    InvalidNumber,
    InvalidLength,
    InvalidKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_string() {
        assert_eq!(
            Bencode::new("5:hello"),
            Ok(Bencode::String("hello".to_string()))
        );
        assert_eq!(Bencode::new("5hello"), Err(BencodeError::MissingDelimeter));
        assert_eq!(Bencode::new("5a:hello"), Err(BencodeError::InvalidNumber));
        assert_eq!(Bencode::new("6:hello"), Err(BencodeError::InvalidLength));
    }

    #[test]
    fn decodes_number() {
        assert_eq!(Bencode::new("i52e"), Ok(Bencode::Number(52)));
        assert_eq!(Bencode::new("i-52e"), Ok(Bencode::Number(-52)));
        assert_eq!(Bencode::new("i52"), Err(BencodeError::MissingDelimeter));
        assert_eq!(Bencode::new("i52ae"), Err(BencodeError::InvalidNumber));
    }

    #[test]
    fn decodes_list() {
        assert_eq!(
            Bencode::new("l5:helloi52ee"),
            Ok(Bencode::List(vec![
                Bencode::String("hello".to_string()),
                Bencode::Number(52)
            ]))
        );
        assert_eq!(
            Bencode::new("l5:helloi52e"),
            Err(BencodeError::MissingDelimeter)
        );
        assert_eq!(
            Bencode::new("l5a:helloi52ee"),
            Err(BencodeError::InvalidNumber)
        );
        assert_eq!(
            Bencode::new("l5:helloi52aee"),
            Err(BencodeError::InvalidNumber)
        );
    }

    #[test]
    fn decodes_dictionary() {
        assert_eq!(
            Bencode::new("d3:foo3:bar5:helloi52ee"),
            Ok(Bencode::Dictionary(HashMap::from([
                ("foo".to_string(), Bencode::String("bar".to_string())),
                ("hello".to_string(), Bencode::Number(52))
            ])))
        );
        assert_eq!(
            Bencode::new("d3:foo3:bar5:helloi52e"),
            Err(BencodeError::MissingDelimeter)
        );
        assert_eq!(
            Bencode::new("d3:foo3:bari52e5:hello"),
            Err(BencodeError::InvalidKey)
        );
        assert_eq!(
            Bencode::new("d3a:foo3:bar5:helloi52ee"),
            Err(BencodeError::InvalidNumber)
        );
        assert_eq!(
            Bencode::new("d3:foo3:bar5:helloi52aee"),
            Err(BencodeError::InvalidNumber)
        );

        // {"inner_dict":{"key1":"value1","key2":42,"list_key":["item1","item2",3]}}
        assert_eq!(
            Bencode::new("d10:inner_dictd4:key16:value14:key2i42e8:list_keyl5:item15:item2i3eeee"),
            Ok(Bencode::Dictionary(HashMap::from([(
                "inner_dict".to_string(),
                Bencode::Dictionary(HashMap::from([
                    ("key1".to_string(), Bencode::String("value1".to_string())),
                    ("key2".to_string(), Bencode::Number(42)),
                    (
                        "list_key".to_string(),
                        Bencode::List(vec![
                            Bencode::String("item1".to_string()),
                            Bencode::String("item2".to_string()),
                            Bencode::Number(3),
                        ])
                    ),
                ]))
            ),])))
        );
    }
}
