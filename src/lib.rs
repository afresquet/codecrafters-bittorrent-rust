use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum Bencode {
    String(String),
    Number(isize),
}

impl Bencode {
    pub fn new(encoded_value: &str) -> Result<Self, BencodeError> {
        match encoded_value.chars().next() {
            Some(c) if c.is_ascii_digit() => {
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
            _ => todo!("Not implemented yet"),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Bencode::String(string) => Value::String(string.to_owned()),
            Bencode::Number(number) => Value::Number((*number).into()),
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
}
