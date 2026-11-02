use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

#[derive(Clone, Debug)]
pub(crate) enum PropertyValue {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Character(char),
    String(String),
    Object(Rc<RefCell<BTreeMap<String, PropertyValue>>>),
    Array(Vec<PropertyValue>),
    Null,
}

#[derive(Default)]
pub(crate) struct ObjectStore {
    objects: HashMap<i64, Rc<RefCell<BTreeMap<String, PropertyValue>>>>,
    next_handle: i64,
}

impl ObjectStore {
    pub(crate) fn create(&mut self) -> i64 {
        let handle = self.next_handle;
        self.next_handle = self.next_handle.saturating_add(1);
        self.objects
            .insert(handle, Rc::new(RefCell::new(BTreeMap::new())));
        handle
    }

    pub(crate) fn create_json(&mut self, json: &str, maximum_values: usize) -> i64 {
        let object = parse_json_object(json, maximum_values).unwrap_or_default();
        self.insert(object)
    }

    pub(crate) fn create_xml(&mut self, xml: &str, maximum_values: usize) -> i64 {
        let object = parse_xml_object(xml, maximum_values).unwrap_or_default();
        self.insert(object)
    }

    fn insert(&mut self, object: BTreeMap<String, PropertyValue>) -> i64 {
        let handle = self.next_handle;
        self.next_handle = self.next_handle.saturating_add(1);
        self.objects.insert(handle, Rc::new(RefCell::new(object)));
        handle
    }

    pub(crate) fn set(
        &mut self,
        handle: i64,
        name: &str,
        value: PropertyValue,
    ) -> Result<(), String> {
        self.get_object(handle)?
            .borrow_mut()
            .insert(name.to_owned(), value);
        Ok(())
    }

    pub(crate) fn get(&self, handle: i64, name: &str) -> Result<PropertyValue, String> {
        self.get_object(handle)?
            .borrow()
            .get(name)
            .cloned()
            .ok_or_else(|| {
                missing_property(
                    name,
                    self.get_object(handle)
                        .map(|object| object.borrow().keys().cloned().collect::<Vec<_>>())
                        .unwrap_or_default(),
                )
            })
    }

    pub(crate) fn get_from_array(
        &self,
        handle: i64,
        name: &str,
        index: usize,
    ) -> Result<PropertyValue, String> {
        let property = self.get(handle, name)?;
        let PropertyValue::Array(values) = property else {
            return Err(format!("a propriedade '{name}' não é um vetor"));
        };
        values
            .get(index)
            .cloned()
            .ok_or_else(|| format!("índice {index} inválido para o vetor da propriedade '{name}'"))
    }

    pub(crate) fn array_len(&self, handle: i64, name: &str) -> Result<usize, String> {
        match self.get(handle, name)? {
            PropertyValue::Array(values) => Ok(values.len()),
            _ => Err(format!("a propriedade '{name}' não é um vetor")),
        }
    }

    pub(crate) fn nested_object(&mut self, value: PropertyValue) -> Result<i64, String> {
        let PropertyValue::Object(object) = value else {
            return Err("o tipo da propriedade não é objeto".to_owned());
        };
        let handle = self.next_handle;
        self.next_handle = self.next_handle.saturating_add(1);
        self.objects.insert(handle, object);
        Ok(handle)
    }

    pub(crate) fn contains(&self, handle: i64, name: &str) -> Result<bool, String> {
        Ok(self.get_object(handle)?.borrow().contains_key(name))
    }

    pub(crate) fn type_code(&self, handle: i64, name: &str) -> Result<i64, String> {
        Ok(match self.get(handle, name)? {
            PropertyValue::Integer(_) => 1,
            PropertyValue::String(value) if value.encode_utf16().count() == 1 => 3,
            PropertyValue::String(_) => 2,
            PropertyValue::Character(_) => 3,
            PropertyValue::Real(_) => 4,
            PropertyValue::Logical(_) => 5,
            PropertyValue::Object(_) => 6,
            PropertyValue::Array(_) => 7,
            PropertyValue::Null => 0,
        })
    }

    pub(crate) fn json(&self, handle: i64) -> Result<String, String> {
        let object = self.get_object(handle)?;
        let mut output = String::new();
        write_json(&PropertyValue::Object(object.clone()), 0, &mut output);
        Ok(output)
    }

    pub(crate) fn release(&mut self, handle: i64) -> Result<(), String> {
        self.objects
            .remove(&handle)
            .map(|_| ())
            .ok_or_else(|| "endereço de objeto inválido".to_owned())
    }

    pub(crate) fn release_all(&mut self) {
        self.objects.clear();
    }

    fn get_object(
        &self,
        handle: i64,
    ) -> Result<&Rc<RefCell<BTreeMap<String, PropertyValue>>>, String> {
        self.objects
            .get(&handle)
            .ok_or_else(|| "endereço de objeto inválido".to_owned())
    }
}

fn missing_property(name: &str, properties: Vec<String>) -> String {
    if properties.is_empty() {
        format!("o objeto não contém a propriedade '{name}'")
    } else {
        format!(
            "o objeto não contém a propriedade '{name}'. Propriedades disponíveis: {}",
            properties.join(", ")
        )
    }
}

fn parse_json_object(
    json: &str,
    maximum_values: usize,
) -> Result<BTreeMap<String, PropertyValue>, ()> {
    let mut parser = JsonParser {
        source: json,
        position: 0,
        values_seen: 0,
        maximum_values,
    };
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.position != json.len() {
        return Err(());
    }
    match value {
        PropertyValue::Object(object) => Ok(object.borrow().clone()),
        _ => Err(()),
    }
}

struct JsonParser<'a> {
    source: &'a str,
    position: usize,
    values_seen: usize,
    maximum_values: usize,
}

impl JsonParser<'_> {
    fn parse_value(&mut self, depth: usize) -> Result<PropertyValue, ()> {
        self.skip_whitespace();
        if depth > 128 {
            return Err(());
        }
        self.values_seen = self.values_seen.checked_add(1).ok_or(())?;
        if self.values_seen > self.maximum_values {
            return Err(());
        }
        match self.peek()? {
            b'{' => self
                .parse_object(depth + 1)
                .map(|object| PropertyValue::Object(Rc::new(RefCell::new(object)))),
            b'[' => self.parse_array(depth + 1).map(PropertyValue::Array),
            b'"' => self.parse_string().map(PropertyValue::String),
            b't' => {
                self.consume_literal(b"true")?;
                Ok(PropertyValue::Logical(true))
            }
            b'f' => {
                self.consume_literal(b"false")?;
                Ok(PropertyValue::Logical(false))
            }
            b'n' => {
                self.consume_literal(b"null")?;
                Ok(PropertyValue::Null)
            }
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err(()),
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<BTreeMap<String, PropertyValue>, ()> {
        self.position += 1;
        self.skip_whitespace();
        let mut object = BTreeMap::new();
        if self.consume_if(b'}') {
            return Ok(object);
        }
        loop {
            self.skip_whitespace();
            let name = self.parse_string()?;
            self.skip_whitespace();
            self.consume(b':')?;
            let value = self.parse_value(depth)?;
            object.insert(name, value);
            self.skip_whitespace();
            if self.consume_if(b'}') {
                return Ok(object);
            }
            self.consume(b',')?;
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<Vec<PropertyValue>, ()> {
        self.position += 1;
        self.skip_whitespace();
        let mut array = Vec::new();
        if self.consume_if(b']') {
            return Ok(array);
        }
        loop {
            array.push(self.parse_value(depth)?);
            self.skip_whitespace();
            if self.consume_if(b']') {
                return Ok(array);
            }
            self.consume(b',')?;
        }
    }

    fn parse_string(&mut self) -> Result<String, ()> {
        self.consume(b'"')?;
        let mut result = String::new();
        loop {
            let byte = *self.source.as_bytes().get(self.position).ok_or(())?;
            match byte {
                b'"' => {
                    self.position += 1;
                    return Ok(result);
                }
                b'\\' => {
                    self.position += 1;
                    let escaped = *self.source.as_bytes().get(self.position).ok_or(())?;
                    self.position += 1;
                    match escaped {
                        b'"' => result.push('"'),
                        b'\\' => result.push('\\'),
                        b'/' => result.push('/'),
                        b'b' => result.push('\u{0008}'),
                        b'f' => result.push('\u{000c}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            let first = self.parse_hex_quad()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if self.source.as_bytes().get(self.position..self.position + 2)
                                    != Some(b"\\u")
                                {
                                    return Err(());
                                }
                                self.position += 2;
                                let second = self.parse_hex_quad()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(());
                                }
                                0x10000
                                    + ((u32::from(first) - 0xD800) << 10)
                                    + (u32::from(second) - 0xDC00)
                            } else if (0xDC00..=0xDFFF).contains(&first) {
                                return Err(());
                            } else {
                                u32::from(first)
                            };
                            result.push(char::from_u32(scalar).ok_or(())?);
                        }
                        _ => return Err(()),
                    }
                }
                0x00..=0x1F => return Err(()),
                _ => {
                    let character = self.source[self.position..].chars().next().ok_or(())?;
                    result.push(character);
                    self.position += character.len_utf8();
                }
            }
        }
    }

    fn parse_hex_quad(&mut self) -> Result<u16, ()> {
        let end = self.position.checked_add(4).ok_or(())?;
        let digits = self.source.get(self.position..end).ok_or(())?;
        let value = u16::from_str_radix(digits, 16).map_err(|_| ())?;
        self.position = end;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<PropertyValue, ()> {
        let start = self.position;
        self.consume_if(b'-');
        if self.consume_if(b'0') {
            if self.peek().is_ok_and(|byte| byte.is_ascii_digit()) {
                return Err(());
            }
        } else {
            self.consume_digits()?;
        }
        let mut real = false;
        if self.consume_if(b'.') {
            real = true;
            self.consume_digits()?;
        }
        if self.consume_if(b'e') || self.consume_if(b'E') {
            real = true;
            if !self.consume_if(b'+') {
                self.consume_if(b'-');
            }
            self.consume_digits()?;
        }
        let text = self.source.get(start..self.position).ok_or(())?;
        if real {
            let value = text.parse::<f64>().map_err(|_| ())?;
            value
                .is_finite()
                .then_some(PropertyValue::Real(value))
                .ok_or(())
        } else {
            text.parse::<i64>()
                .map(PropertyValue::Integer)
                .map_err(|_| ())
        }
    }

    fn consume_digits(&mut self) -> Result<(), ()> {
        let start = self.position;
        while self.peek().is_ok_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
        if start == self.position {
            Err(())
        } else {
            Ok(())
        }
    }

    fn consume_literal(&mut self, literal: &[u8]) -> Result<(), ()> {
        if self
            .source
            .as_bytes()
            .get(self.position..self.position + literal.len())
            != Some(literal)
        {
            return Err(());
        }
        self.position += literal.len();
        Ok(())
    }

    fn consume(&mut self, expected: u8) -> Result<(), ()> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.source.as_bytes().get(self.position) == Some(&expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Result<u8, ()> {
        self.source.as_bytes().get(self.position).copied().ok_or(())
    }

    fn skip_whitespace(&mut self) {
        while self
            .peek()
            .is_ok_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.position += 1;
        }
    }
}

fn write_json(value: &PropertyValue, depth: usize, output: &mut String) {
    match value {
        PropertyValue::Integer(value) => output.push_str(&value.to_string()),
        PropertyValue::Real(value) => output.push_str(&value.to_string()),
        PropertyValue::Logical(value) => output.push_str(if *value { "true" } else { "false" }),
        PropertyValue::Character(value) => write_json_string(&value.to_string(), output),
        PropertyValue::String(value) => write_json_string(value, output),
        PropertyValue::Object(object) => {
            let object = object.borrow();
            if object.is_empty() {
                output.push_str("{}");
                return;
            }
            output.push_str("{\n");
            for (index, (key, value)) in object.iter().enumerate() {
                write_indent(depth + 1, output);
                write_json_string(key, output);
                output.push_str(": ");
                write_json(value, depth + 1, output);
                if index + 1 < object.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            write_indent(depth, output);
            output.push('}');
        }
        PropertyValue::Array(values) => {
            if values.is_empty() {
                output.push_str("[]");
                return;
            }
            output.push_str("[\n");
            for (index, value) in values.iter().enumerate() {
                write_indent(depth + 1, output);
                write_json(value, depth + 1, output);
                if index + 1 < values.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            write_indent(depth, output);
            output.push(']');
        }
        PropertyValue::Null => output.push_str("null"),
    }
}

fn write_json_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{001F}' => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn write_indent(depth: usize, output: &mut String) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn parse_xml_object(
    xml: &str,
    maximum_values: usize,
) -> Result<BTreeMap<String, PropertyValue>, ()> {
    let mut parser = XmlParser {
        source: xml,
        position: 0,
        values_seen: 0,
        maximum_values,
    };
    parser.skip_misc()?;
    let (name, value) = parser.parse_element(0)?;
    parser.skip_misc()?;
    if parser.position != xml.len() {
        return Err(());
    }
    Ok(BTreeMap::from([(name, value)]))
}

struct XmlParser<'a> {
    source: &'a str,
    position: usize,
    values_seen: usize,
    maximum_values: usize,
}

impl XmlParser<'_> {
    fn parse_element(&mut self, depth: usize) -> Result<(String, PropertyValue), ()> {
        if depth > 128 {
            return Err(());
        }
        self.values_seen = self.values_seen.checked_add(1).ok_or(())?;
        if self.values_seen > self.maximum_values {
            return Err(());
        }
        self.consume_byte(b'<')?;
        let name = self.read_name()?;
        let (attributes, self_closing) = self.read_attributes()?;
        let mut children = attributes;
        if self_closing {
            let value = if children.is_empty() {
                PropertyValue::String(String::new())
            } else {
                PropertyValue::Object(Rc::new(RefCell::new(children)))
            };
            return Ok((name, value));
        }
        self.consume_byte(b'>')?;
        let mut text = String::new();
        loop {
            if self.position >= self.source.len() {
                return Err(());
            }
            if self.remaining().starts_with("</") {
                self.position += 2;
                let closing_name = self.read_name()?;
                if closing_name != name {
                    return Err(());
                }
                self.skip_whitespace();
                self.consume_byte(b'>')?;
                break;
            }
            if self.remaining().starts_with("<!--") {
                self.skip_until("-->")?;
                continue;
            }
            if self.remaining().starts_with("<![CDATA[") {
                self.skip_until("<![CDATA[")?;
                let end = self.source[self.position..].find("]]>").ok_or(())?;
                text.push_str(&self.source[self.position..self.position + end]);
                self.position += end + 3;
                continue;
            }
            if self.remaining().starts_with('<') {
                let (child_name, child_value) = self.parse_element(depth + 1)?;
                match children.remove(&child_name) {
                    Some(PropertyValue::Array(mut values)) => {
                        values.push(child_value);
                        children.insert(child_name, PropertyValue::Array(values));
                    }
                    Some(previous) => {
                        children.insert(
                            child_name,
                            PropertyValue::Array(vec![previous, child_value]),
                        );
                    }
                    None => {
                        children.insert(child_name, child_value);
                    }
                }
                continue;
            }
            let end = self.remaining().find('<').ok_or(())?;
            text.push_str(&decode_xml_entities(
                &self.source[self.position..self.position + end],
            )?);
            self.position += end;
        }
        let value = if children.is_empty() {
            PropertyValue::String(text.trim().to_owned())
        } else {
            if !text.trim().is_empty() {
                children.insert(String::new(), PropertyValue::String(text.trim().to_owned()));
            }
            PropertyValue::Object(Rc::new(RefCell::new(children)))
        };
        Ok((name, value))
    }

    fn skip_misc(&mut self) -> Result<(), ()> {
        loop {
            self.skip_whitespace();
            if self.remaining().starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.remaining().starts_with("<!--") {
                self.skip_until("-->")?;
            } else {
                return Ok(());
            }
        }
    }

    fn read_attributes(&mut self) -> Result<(BTreeMap<String, PropertyValue>, bool), ()> {
        let mut attributes = BTreeMap::new();
        loop {
            self.skip_whitespace();
            match self.remaining().as_bytes().first().copied() {
                Some(b'>') => return Ok((attributes, false)),
                Some(b'/') => {
                    self.position += 1;
                    self.consume_byte(b'>')?;
                    return Ok((attributes, true));
                }
                Some(_) => {
                    let name = self.read_name()?;
                    self.skip_whitespace();
                    self.consume_byte(b'=')?;
                    self.skip_whitespace();
                    let quote = self.remaining().as_bytes().first().copied().ok_or(())?;
                    if quote != b'\'' && quote != b'"' {
                        return Err(());
                    }
                    self.position += 1;
                    let end = self.source[self.position..]
                        .as_bytes()
                        .iter()
                        .position(|byte| *byte == quote)
                        .ok_or(())?;
                    let value =
                        decode_xml_entities(&self.source[self.position..self.position + end])?;
                    self.position += end + 1;
                    if name != "xmlns" && !name.starts_with("xmlns:") {
                        self.values_seen = self.values_seen.checked_add(1).ok_or(())?;
                        if self.values_seen > self.maximum_values {
                            return Err(());
                        }
                        attributes.insert(name, PropertyValue::String(value));
                    }
                }
                None => return Err(()),
            }
        }
    }

    fn read_name(&mut self) -> Result<String, ()> {
        let start = self.position;
        while self.remaining().as_bytes().first().is_some_and(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.')
        }) {
            self.position += 1;
        }
        if self.position == start {
            Err(())
        } else {
            Ok(self.source[start..self.position].to_owned())
        }
    }

    fn skip_until(&mut self, marker: &str) -> Result<(), ()> {
        let end = self.remaining().find(marker).ok_or(())?;
        self.position += end + marker.len();
        Ok(())
    }

    fn consume_byte(&mut self, expected: u8) -> Result<(), ()> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.remaining().as_bytes().first() == Some(&expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .remaining()
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn remaining(&self) -> &str {
        &self.source[self.position..]
    }
}

fn decode_xml_entities(text: &str) -> Result<String, ()> {
    let mut result = String::new();
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('&') {
        result.push_str(before);
        let (entity, remaining) = after.split_once(';').ok_or(())?;
        let decoded = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            entity if entity.starts_with("#x") => {
                xml_character(u32::from_str_radix(&entity[2..], 16).map_err(|_| ())?)?
            }
            entity if entity.starts_with('#') => {
                xml_character(entity[1..].parse::<u32>().map_err(|_| ())?)?
            }
            _ => return Err(()),
        };
        result.push(decoded);
        rest = remaining;
    }
    result.push_str(rest);
    Ok(result)
}

fn xml_character(value: u32) -> Result<char, ()> {
    if matches!(value, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF) {
        char::from_u32(value).ok_or(())
    } else {
        Err(())
    }
}
