use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum Bencode {
    String(String),
    Number(isize),
    List(Vec<Bencode>),
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
            _ => todo!(),
        }
    }

    fn encoded_length(&self) -> usize {
        match self {
            Bencode::String(s) => s.len() + 2,
            Bencode::Number(n) => n.to_string().len() + 2,
            Bencode::List(l) => l.iter().map(|v| v.encoded_length()).sum::<usize>() + 2,
        }
    }
}

impl From<&Bencode> for Value {
    fn from(value: &Bencode) -> Self {
        match value {
            Bencode::String(string) => Value::String(string.to_owned()),
            Bencode::Number(number) => Value::Number((*number).into()),
            Bencode::List(values) => Value::Array(values.iter().map(Into::into).collect()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BencodeError {
    MissingDelimeter,
    InvalidNumber,
    InvalidLength,
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
}
