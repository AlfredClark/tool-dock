//! 数据互转业务逻辑（工具路由 `text/convert`）：六格式经 `serde_json::Value` 中转的纯函数实现。
//!
//! 六格式双向全通：JSON/YAML/TOML/Properties 为原生映射；XML 属性收进 `@键`（有属性又有文本时
//! 文本收进 `#text`），混合内容（子元素间的非空文本）直接报错；INI 以 section 建嵌套、重复键收成数组。
//! XML/INI/Properties 解析出的值全是字符串（无类型信息），数字往返会变成字符串，是已知局限。
//! 业务失败全部装进 [`ConvertOutcome`] 返回，不抛错（输入纠错是常态 UI 状态，不是异常）。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

/// 单次转换输入上限：1 MiB，超限直接拒收（错误码 `TooLarge`），不进解析器
const MAX_INPUT_LEN: usize = 1024 * 1024;

/// 可转换的数据格式：六变体一次定全，稳定前后端契约（线上传 `lowercase`，与前端下拉取值一致）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ConvertFormat {
    Json,
    Yaml,
    Toml,
    Xml,
    Ini,
    Properties,
}

/// JSON 缩进选项（输出端高级选项；XML 缩进复用同一枚举）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum JsonIndent {
    Two,
    Four,
    Tab,
}

/// INI 键值分隔符（输出端高级选项）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum IniKvSeparator {
    Compact,
    Spaced,
}

/// 转换选项（输出端高级选项；无选项的格式忽略对应字段）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConvertOptions {
    pub json_indent: JsonIndent,
    pub xml_root_name: String,
    pub xml_declaration: bool,
    pub xml_indent: JsonIndent,
    pub ini_kv_separator: IniKvSeparator,
    pub properties_escape_unicode: bool,
    pub xml_trailing_newline: bool,
}

/// 业务错误码：前端按码映射 i18n 文案，`format`/`message` 只做诊断补充（解析器原文，保持英文）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum ErrorCode {
    TooLarge,
    UnknownFormat,
    ParseFailed,
    UnsupportedInput,
    UnsupportedOutput,
    NonTableRoot,
    UnsupportedValue,
}

/// 业务错误体：扁平结构便于 `specta` 导出，前端按 `code` 分支；
/// `line`/`column` 为 1 基位置（解析器给不出时为 `None`，前端仅有值时渲染"第 N 行"）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConvertError {
    pub code: ErrorCode,
    pub format: Option<ConvertFormat>,
    pub message: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// 解析失败详情（内部类型）：原文 + 结构化位置，由各解析器映射函数填充
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParseFailure {
    message: String,
    line: Option<u32>,
    column: Option<u32>,
}

impl ParseFailure {
    /// 无位置信息的失败（如值域错误、输出约束）
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// 带解析器行号的失败：行列统一为 1 基 `u32`（各库约定不一，在此收敛）
    fn at(message: impl Into<String>, line: usize, column: Option<usize>) -> Self {
        Self {
            message: message.into(),
            line: Some(saturating_u32(line)),
            column: column.map(saturating_u32),
        }
    }
}

/// `usize` 转契约 `u32`：输入有 1 MiB 上限，转换必成功，保底防溢出
fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// 序列化阶段的失败元组构造：调用方装进 `ConvertOutcome`
fn ser_fail(code: ErrorCode, message: impl Into<String>) -> (ErrorCode, ParseFailure) {
    (code, ParseFailure::new(message))
}

/// 转换结果：`ok` 为真时读 `output`/`detected`，为假时读 `error`（输出端保留上次成功内容）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConvertOutcome {
    pub ok: bool,
    pub output: String,
    pub detected: Option<ConvertFormat>,
    pub error: Option<ConvertError>,
}

impl ConvertOutcome {
    const fn success(output: String, detected: Option<ConvertFormat>) -> Self {
        Self {
            ok: true,
            output,
            detected,
            error: None,
        }
    }

    fn failure(
        code: ErrorCode,
        format: Option<ConvertFormat>,
        detail: Option<ParseFailure>,
    ) -> Self {
        let (message, line, column) = match detail {
            Some(detail) => (Some(detail.message), detail.line, detail.column),
            None => (None, None, None),
        };
        Self {
            ok: false,
            output: String::new(),
            detected: format,
            error: Some(ConvertError {
                code,
                format,
                message,
                line,
                column,
            }),
        }
    }
}

/// 字节偏移转 1 基行列号（`quick-xml` 与 TOML 只给字节偏移时用）：
/// 切片按字符边界保护，列号按字符计（中文行的字节列会误导跳转）
fn line_col_of(input: &str, byte_offset: u64) -> (u32, u32) {
    let offset = usize::try_from(byte_offset)
        .unwrap_or(usize::MAX)
        .min(input.len());
    let prefix = input.get(..offset).unwrap_or(input);
    let line = prefix.matches('\n').count() + 1;
    let column = prefix
        .rfind('\n')
        .map_or(prefix, |pos| &prefix[pos + 1..])
        .chars()
        .count()
        + 1;
    (saturating_u32(line), saturating_u32(column))
}

/// 自动识别输入格式：JSON（严格）→ XML（`^<` 特征）→ TOML → YAML（结构化）→ INI → Properties → YAML（标量兜底）。
///
/// YAML 是 JSON 的超集且纯文本行也是合法 YAML 标量，所以分两段认领：能解析出对象/数组的
/// 优先判 YAML（`a: 1` 这类映射不会被 Properties 的 `:` 分隔符门控误收）；纯标量（如多行
/// `k=v` 文本）放行给 INI/Properties，最后仍无归属才回落 YAML；
/// INI 要求出现 section 头（无 section 的扁平键值与 Properties 无从区分，归 Properties）；
/// 简单的 `k=v` 内容会被判成 TOML（两者解释一致，结果相同，是良性行为）。
pub fn detect_format(text: &str) -> Option<ConvertFormat> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if serde_json::from_str::<Value>(trimmed).is_ok() {
        return Some(ConvertFormat::Json);
    }
    if trimmed.starts_with('<') {
        return Some(ConvertFormat::Xml);
    }
    if toml::from_str::<toml::Value>(trimmed).is_ok() {
        return Some(ConvertFormat::Toml);
    }
    if let Ok(value) = serde_yml::from_str::<Value>(trimmed)
        && (value.is_object() || value.is_array())
    {
        return Some(ConvertFormat::Yaml);
    }
    if is_ini_like(trimmed) {
        return Some(ConvertFormat::Ini);
    }
    if is_properties_like(trimmed) {
        return Some(ConvertFormat::Properties);
    }
    if serde_yml::from_str::<Value>(trimmed).is_ok() {
        return Some(ConvertFormat::Yaml);
    }
    None
}

/// INI 判定：必须出现 section 头（`[name]` 行）且解析出实质键值；
/// 无 section 的扁平键值归 Properties，避免两者在无 section 输入上永远冲突
fn is_ini_like(text: &str) -> bool {
    let has_section = text.lines().any(|line| {
        let line = line.trim();
        line.starts_with('[') && line.contains(']')
    });
    has_section
        && ini::Ini::load_from_str(text)
            .is_ok_and(|ini| ini.iter().any(|(_, props)| !props.is_empty()))
}

/// Properties 判定：必须含 `=`/`:` 分隔符（裸词行按 properties 语义是空值键，会误收垃圾输入），
/// 且解析结果非空；`{{{` 这类输入到此为止，保证 `UnknownFormat` 不变
fn is_properties_like(text: &str) -> bool {
    if !text.contains(['=', ':']) {
        return false;
    }
    java_properties::read(text.as_bytes())
        .is_ok_and(|map: std::collections::HashMap<String, String>| !map.is_empty())
}

/// 执行转换：`from` 为 `None` 即自动识别；同格式输入同样走解析-序列化（等于按选项美化）。
pub fn convert(
    input: &str,
    from: Option<ConvertFormat>,
    to: ConvertFormat,
    options: &ConvertOptions,
) -> ConvertOutcome {
    if input.len() > MAX_INPUT_LEN {
        return ConvertOutcome::failure(ErrorCode::TooLarge, None, None);
    }
    if input.trim().is_empty() {
        return ConvertOutcome::success(String::new(), None);
    }
    let source = match from {
        Some(format) => format,
        None => match detect_format(input) {
            Some(format) => format,
            None => return ConvertOutcome::failure(ErrorCode::UnknownFormat, None, None),
        },
    };
    let value = match parse_input(input, source) {
        Ok(value) => value,
        Err(detail) => {
            return ConvertOutcome::failure(ErrorCode::ParseFailed, Some(source), Some(detail));
        }
    };
    serialize_output(&value, to, options).map_or_else(
        |(code, detail)| ConvertOutcome::failure(code, Some(to), Some(detail)),
        |output| ConvertOutcome::success(output, (from.is_none()).then_some(source)),
    )
}

/// 各输入解析为 IR；TOML 时间类型转字符串（`toml::Value` 直转 JSON 会丢语义）
fn parse_input(input: &str, source: ConvertFormat) -> Result<Value, ParseFailure> {
    match source {
        ConvertFormat::Json => serde_json::from_str(input).map_err(|err: serde_json::Error| {
            ParseFailure::at(err.to_string(), err.line(), Some(err.column()))
        }),
        ConvertFormat::Yaml => serde_yml::from_str(input).map_err(|err| {
            err.location().map_or_else(
                || ParseFailure::new(err.to_string()),
                |location| {
                    ParseFailure::at(err.to_string(), location.line(), Some(location.column()))
                },
            )
        }),
        ConvertFormat::Toml => {
            let toml_value: toml::Value = toml::from_str(input).map_err(|err| {
                let (line, column) = err.span().map_or((1, 1), |span| {
                    line_col_of(input, u64::try_from(span.start).unwrap_or(u64::MAX))
                });
                ParseFailure {
                    message: err.to_string(),
                    line: Some(line),
                    column: Some(column),
                }
            })?;
            toml_value_to_json(&toml_value).map_err(ParseFailure::new)
        }
        ConvertFormat::Xml => parse_xml(input),
        ConvertFormat::Ini => parse_ini(input),
        ConvertFormat::Properties => parse_properties(input),
    }
}

/// TOML 值转 IR：时间按 RFC3339 文本保留，其余直映
fn toml_value_to_json(value: &toml::Value) -> Result<Value, String> {
    match value {
        toml::Value::String(v) => Ok(Value::String(v.clone())),
        toml::Value::Integer(v) => Ok(Value::Number((*v).into())),
        toml::Value::Float(v) => serde_json::Number::from_f64(*v)
            .map(Value::Number)
            .ok_or_else(|| "non-finite float is not representable in JSON".to_string()),
        toml::Value::Boolean(v) => Ok(Value::Bool(*v)),
        toml::Value::Datetime(v) => Ok(Value::String(v.to_string())),
        toml::Value::Array(items) => items
            .iter()
            .map(toml_value_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::with_capacity(table.len());
            for (key, item) in table {
                map.insert(key.clone(), toml_value_to_json(item)?);
            }
            Ok(Value::Object(map))
        }
    }
}

/// IR 序列化为目标格式；失败返回（错误码，诊断信息），由调用方装进 `ConvertOutcome`
fn serialize_output(
    value: &Value,
    to: ConvertFormat,
    options: &ConvertOptions,
) -> Result<String, (ErrorCode, ParseFailure)> {
    match to {
        ConvertFormat::Json => Ok(format_json(value, options.json_indent)),
        ConvertFormat::Yaml => serde_yml::to_string(value)
            .map_err(|err| ser_fail(ErrorCode::ParseFailed, err.to_string())),
        ConvertFormat::Toml => json_to_toml_string(value),
        ConvertFormat::Xml => Ok(value_to_xml(value, options)),
        ConvertFormat::Ini => json_to_ini_string(value, options),
        ConvertFormat::Properties => json_to_properties_string(value, options),
    }
}

/// 缩进选项映射为排版垫片：JSON 与 XML 输出共用
const fn indent_pad(indent: JsonIndent) -> &'static str {
    match indent {
        JsonIndent::Two => "  ",
        JsonIndent::Four => "    ",
        JsonIndent::Tab => "\t",
    }
}

/// JSON 手写排版（`serde_json` 的 pretty 固定两空格，满足不了 4/Tab 选项）：
/// 只处理对象/数组结构，标量与键转义复用 `serde_json`，保证与标准一致
fn format_json(value: &Value, indent: JsonIndent) -> String {
    let mut out = String::new();
    write_json(&mut out, value, 0, indent_pad(indent));
    out
}

fn write_json(out: &mut String, value: &Value, depth: usize, pad: &str) {
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            let mut first = true;
            for (key, item) in map {
                if !first {
                    out.push_str(",\n");
                }
                first = false;
                out.push_str(&pad.repeat(depth + 1));
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push_str(": ");
                write_json(out, item, depth + 1, pad);
            }
            out.push('\n');
            out.push_str(&pad.repeat(depth));
            out.push('}');
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&pad.repeat(depth + 1));
                write_json(out, item, depth + 1, pad);
            }
            out.push('\n');
            out.push_str(&pad.repeat(depth));
            out.push(']');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

/// IR 转 TOML 文本：顶层必须为表（TOML 无顶层数组/标量），`null` 无对应表示，均报明确错误
fn json_to_toml_string(value: &Value) -> Result<String, (ErrorCode, ParseFailure)> {
    let table = match value {
        Value::Object(map) => {
            let mut table = toml::map::Map::with_capacity(map.len());
            for (key, item) in map {
                table.insert(key.clone(), json_to_toml_value(item)?);
            }
            table
        }
        _ => {
            return Err(ser_fail(
                ErrorCode::NonTableRoot,
                format!(
                    "{} requires a top-level table (object)",
                    format_display_name(ConvertFormat::Toml)
                ),
            ));
        }
    };
    toml::to_string(&toml::Value::Table(table))
        .map_err(|err| ser_fail(ErrorCode::ParseFailed, err.to_string()))
}

/// IR 值转 TOML 值：整数超 `i64`、非有限浮点、`null` 都无 TOML 对应，直接报错
fn json_to_toml_value(value: &Value) -> Result<toml::Value, (ErrorCode, ParseFailure)> {
    match value {
        Value::String(v) => Ok(toml::Value::String(v.clone())),
        Value::Bool(v) => Ok(toml::Value::Boolean(*v)),
        Value::Number(v) => match (v.as_i64(), v.as_u64(), v.as_f64()) {
            (Some(int), _, _) => Ok(toml::Value::Integer(int)),
            (None, Some(uint), _) => int_from_u64(uint),
            (None, None, Some(float)) if float.is_finite() => Ok(toml::Value::Float(float)),
            (None, None, Some(_)) => Err(ser_fail(
                ErrorCode::UnsupportedValue,
                "non-finite float has no TOML form",
            )),
            (None, None, None) => Err(ser_fail(
                ErrorCode::UnsupportedValue,
                "number has no TOML form",
            )),
        },
        Value::Array(items) => items
            .iter()
            .map(json_to_toml_value)
            .collect::<Result<Vec<_>, _>>()
            .map(toml::Value::Array),
        Value::Object(map) => {
            let mut table = toml::map::Map::with_capacity(map.len());
            for (key, item) in map {
                table.insert(key.clone(), json_to_toml_value(item)?);
            }
            Ok(toml::Value::Table(table))
        }
        Value::Null => Err(ser_fail(
            ErrorCode::UnsupportedValue,
            "TOML has no null value",
        )),
    }
}

/// `u64` 转 TOML 整数：超 `i64` 上限即报错（静默截断比报错更危险）
fn int_from_u64(value: u64) -> Result<toml::Value, (ErrorCode, ParseFailure)> {
    i64::try_from(value).map(toml::Value::Integer).map_err(|_| {
        ser_fail(
            ErrorCode::UnsupportedValue,
            "integer exceeds TOML i64 range",
        )
    })
}

/// IR 转 XML 文本：对象键→子元素、数组→重复同名元素，按 `xml_indent` 排版；
/// 纯文本元素保持单行（不破坏文本语义），有子元素的才换行缩进
fn value_to_xml(value: &Value, options: &ConvertOptions) -> String {
    let mut out = String::new();
    if options.xml_declaration {
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    }
    write_xml_element(
        &mut out,
        &sanitize_xml_name(&options.xml_root_name),
        value,
        0,
        indent_pad(options.xml_indent),
    );
    // 根元素恒以单个换行结尾；关闭尾换行时去掉它（输出非空是结构保证，无需防御分支）
    if !options.xml_trailing_newline {
        out.pop();
    }
    out
}

fn write_xml_element(out: &mut String, name: &str, value: &Value, depth: usize, pad: &str) {
    let indent = pad.repeat(depth);
    match value {
        Value::Null => {
            out.push_str(&indent);
            out.push('<');
            out.push_str(name);
            out.push_str("/>\n");
        }
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str(&indent);
                out.push('<');
                out.push_str(name);
                out.push_str("/>\n");
                return;
            }
            out.push_str(&indent);
            out.push('<');
            out.push_str(name);
            out.push_str(">\n");
            for (key, item) in map {
                write_xml_element(out, &sanitize_xml_name(key), item, depth + 1, pad);
            }
            out.push_str(&indent);
            out.push_str("</");
            out.push_str(name);
            out.push_str(">\n");
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str(&indent);
                out.push('<');
                out.push_str(name);
                out.push_str("/>\n");
                return;
            }
            for item in items {
                write_xml_element(out, name, item, depth, pad);
            }
        }
        Value::String(text) => {
            out.push_str(&indent);
            out.push('<');
            out.push_str(name);
            out.push('>');
            escape_xml_into(out, text);
            out.push_str("</");
            out.push_str(name);
            out.push_str(">\n");
        }
        Value::Number(_) | Value::Bool(_) => {
            out.push_str(&indent);
            out.push('<');
            out.push_str(name);
            out.push('>');
            out.push_str(&value.to_string());
            out.push_str("</");
            out.push_str(name);
            out.push_str(">\n");
        }
    }
}

/// XML 转义：`&<>"'` 五项，数字与布尔走 `to_string` 本身不含特殊字符
fn escape_xml_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
}

/// 根名/键名清洗：首字符须为字母或下划线（简化规则，不含 Unicode 名），非法回落 `"root"`
fn sanitize_xml_name(name: &str) -> String {
    let mut chars = name.chars();
    let valid_first = chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_' || ch == ':');
    let valid_rest = name
        .chars()
        .skip(1)
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'));
    if !name.is_empty() && valid_first && valid_rest {
        name.to_string()
    } else {
        "root".to_string()
    }
}

/// 格式展示名：诊断信息用（前端用户文案走 i18n，此处保持英文技术文本）
const fn format_display_name(format: ConvertFormat) -> &'static str {
    match format {
        ConvertFormat::Json => "JSON",
        ConvertFormat::Yaml => "YAML",
        ConvertFormat::Toml => "TOML",
        ConvertFormat::Xml => "XML",
        ConvertFormat::Ini => "INI",
        ConvertFormat::Properties => "Properties",
    }
}

/// XML 解析为 IR：属性收进 `@键`，有属性又有文本时文本收进 `#text`；
/// 重复同名子元素收成数组；声明/注释/处理指令跳过；
/// 混合内容（子元素之间的非空文本）直接报错，宁可失败也不静默丢数据。
fn parse_xml(input: &str) -> Result<Value, ParseFailure> {
    use quick_xml::{events::Event, reader::Reader};

    let mut reader = Reader::from_str(input);
    loop {
        let event = match reader.read_event() {
            Ok(event) => event,
            Err(err) => return Err(xml_read_error(input, &reader, &err)),
        };
        match event {
            Event::Start(start) => return element_to_value(&mut reader, &start, input),
            Event::Empty(empty) => return empty_to_value(&reader, &empty),
            Event::Text(text) => {
                if !text
                    .xml_content()
                    .map_err(|err| ParseFailure::new(err.to_string()))?
                    .trim()
                    .is_empty()
                {
                    return Err(xml_pos_error(input, &reader, "text outside root element"));
                }
            }
            Event::CData(_) => {
                return Err(xml_pos_error(input, &reader, "text outside root element"));
            }
            Event::GeneralRef(_) => {
                return Err(xml_pos_error(
                    input,
                    &reader,
                    "reference outside root element",
                ));
            }
            Event::End(_) => {
                return Err(xml_pos_error(input, &reader, "unexpected closing tag"));
            }
            Event::Eof => return Err(ParseFailure::new("empty XML document")),
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }
}

/// `read_event` 失败的位置快照：`error_position` 是字节偏移，转 1 基行列
fn xml_read_error(
    input: &str,
    reader: &quick_xml::reader::Reader<&[u8]>,
    err: &quick_xml::Error,
) -> ParseFailure {
    let (line, column) = line_col_of(input, reader.error_position());
    ParseFailure {
        message: err.to_string(),
        line: Some(line),
        column: Some(column),
    }
}

/// 手写检查项的位置快照：`buffer_position` 是当前已消费字节数，转 1 基行列
fn xml_pos_error(
    input: &str,
    reader: &quick_xml::reader::Reader<&[u8]>,
    message: impl Into<String>,
) -> ParseFailure {
    let (line, column) = line_col_of(input, reader.buffer_position());
    ParseFailure {
        message: message.into(),
        line: Some(line),
        column: Some(column),
    }
}

/// 起始标签名解码：`from_str` 输入保证 UTF-8，直接转字符串
fn element_name(name: &quick_xml::name::QName) -> Result<String, ParseFailure> {
    std::str::from_utf8(name.as_ref())
        .map(str::to_string)
        .map_err(|err| ParseFailure::new(err.to_string()))
}

/// 起始/空标签的属性收成 `@键` 对象
fn attributes_to_map(
    reader: &quick_xml::reader::Reader<&[u8]>,
    element: &quick_xml::events::BytesStart,
) -> Result<serde_json::Map<String, Value>, ParseFailure> {
    let mut attrs = serde_json::Map::new();
    for attr in element.attributes() {
        let attr = attr.map_err(|err| ParseFailure::new(err.to_string()))?;
        let key = format!("@{}", element_name(&attr.key)?);
        let value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(|err| ParseFailure::new(err.to_string()))?;
        attrs.insert(key, Value::String(value.into_owned()));
    }
    Ok(attrs)
}

/// 空元素（`<n/>`）：有属性即属性对象，否则 `Null`
fn empty_to_value(
    reader: &quick_xml::reader::Reader<&[u8]>,
    empty: &quick_xml::events::BytesStart,
) -> Result<Value, ParseFailure> {
    let attrs = attributes_to_map(reader, empty)?;
    if attrs.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::Object(attrs))
    }
}

/// 递归读完一个元素：子元素名冲突（重复标签）时收成数组，保持文档顺序
fn element_to_value(
    reader: &mut quick_xml::reader::Reader<&[u8]>,
    start: &quick_xml::events::BytesStart,
    input: &str,
) -> Result<Value, ParseFailure> {
    use quick_xml::events::Event;

    let tag = element_name(&start.name())?;
    let attrs = attributes_to_map(reader, start)?;
    let mut children: serde_json::Map<String, Value> = serde_json::Map::new();
    let mut text = String::new();
    loop {
        let event = match reader.read_event() {
            Ok(event) => event,
            Err(err) => return Err(xml_read_error(input, reader, &err)),
        };
        match event {
            Event::Start(child) => {
                let child_tag = element_name(&child.name())?;
                let child_value = element_to_value(reader, &child, input)?;
                merge_child(&mut children, child_tag, child_value);
            }
            Event::Empty(child) => {
                let child_tag = element_name(&child.name())?;
                let child_value = empty_to_value(reader, &child)?;
                merge_child(&mut children, child_tag, child_value);
            }
            Event::Text(content) => {
                text.push_str(
                    &content
                        .xml_content()
                        .map_err(|err| ParseFailure::new(err.to_string()))?,
                );
            }
            Event::CData(content) => {
                text.push_str(
                    &content
                        .xml_content()
                        .map_err(|err| ParseFailure::new(err.to_string()))?,
                );
            }
            Event::GeneralRef(reference) => {
                push_general_ref(&mut text, &reference).map_err(ParseFailure::new)?;
            }
            Event::End(end) => {
                let end_tag = element_name(&end.name())?;
                if end_tag != tag {
                    return Err(xml_pos_error(
                        input,
                        reader,
                        format!("mismatched closing tag: expected </{tag}>, found </{end_tag}>"),
                    ));
                }
                break;
            }
            Event::Eof => {
                return Err(xml_pos_error(
                    input,
                    reader,
                    format!("unclosed tag <{tag}>"),
                ));
            }
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }
    assemble_element(attrs, children, &text)
        .map_err(|message| xml_pos_error(input, reader, message))
}

/// 独立实体引用（如文本中的 `&amp;` 被 reader 拆成单独事件）解码为字符；
/// 未知具名实体直接报错（不猜测 DTD 定义）
fn push_general_ref(
    text: &mut String,
    reference: &quick_xml::events::BytesRef,
) -> Result<(), String> {
    if let Some(ch) = reference
        .resolve_char_ref()
        .map_err(|err| err.to_string())?
    {
        text.push(ch);
        return Ok(());
    }
    let name = reference.decode().map_err(|err| err.to_string())?;
    match name.as_ref() {
        "amp" => text.push('&'),
        "lt" => text.push('<'),
        "gt" => text.push('>'),
        "quot" => text.push('"'),
        "apos" => text.push('\''),
        _ => return Err(format!("unknown entity reference &{name};")),
    }
    Ok(())
}

/// 同名子元素合并：第二次出现即升级为数组（保序）
fn merge_child(children: &mut serde_json::Map<String, Value>, tag: String, value: Value) {
    match children.remove(&tag) {
        None => {
            children.insert(tag, value);
        }
        Some(Value::Array(mut items)) => {
            items.push(value);
            children.insert(tag, Value::Array(items));
        }
        Some(first) => {
            children.insert(tag, Value::Array(vec![first, value]));
        }
    }
}

/// 元素组装：纯文本直出字符串；有子元素时忽略纯空白文本，非空文本即混合内容报错；
/// 属性（`@键`）与子元素合并（元素名不可能含 `@`，不会冲突）
fn assemble_element(
    attrs: serde_json::Map<String, Value>,
    children: serde_json::Map<String, Value>,
    text: &str,
) -> Result<Value, String> {
    if children.is_empty() {
        if text.is_empty() {
            return if attrs.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(Value::Object(attrs))
            };
        }
        if attrs.is_empty() {
            return Ok(Value::String(text.to_string()));
        }
        let mut obj = attrs;
        obj.insert("#text".to_string(), Value::String(text.to_string()));
        return Ok(Value::Object(obj));
    }
    if !text.trim().is_empty() {
        return Err("mixed content (text between child elements) is not supported".to_string());
    }
    let mut obj = attrs;
    obj.extend(children);
    Ok(Value::Object(obj))
}

/// INI 解析为 IR：section 建嵌套对象，无 section 的键进顶层；重复键收成数组（保序）
fn parse_ini(input: &str) -> Result<Value, ParseFailure> {
    // `rust-ini` 的行号是 0 基（换行计数），转 1 基；列号已是 1 基，原样透传
    let ini = ini::Ini::load_from_str(input)
        .map_err(|err| ParseFailure::at(err.to_string(), err.line + 1, Some(err.col)))?;
    let mut root = serde_json::Map::new();
    for (section, props) in &ini {
        let mut grouped: Vec<(String, Vec<String>)> = Vec::new();
        for (key, value) in props {
            match grouped.iter_mut().find(|(existing, _)| existing == key) {
                Some((_, values)) => values.push(value.to_string()),
                None => grouped.push((key.to_string(), vec![value.to_string()])),
            }
        }
        let mut obj = serde_json::Map::with_capacity(grouped.len());
        for (key, values) in grouped {
            obj.insert(
                key,
                if values.len() == 1 {
                    Value::String(values.into_iter().next().unwrap_or_default())
                } else {
                    Value::Array(values.into_iter().map(Value::String).collect())
                },
            );
        }
        match section {
            Some(name) => {
                root.insert(name.to_string(), Value::Object(obj));
            }
            None => root.extend(obj),
        }
    }
    Ok(Value::Object(root))
}

/// IR 转 INI 文本：顶层必须为对象；section 只下一层（更深嵌套报错）；
/// 标量原样/转字符串，`null` 无表示；数组写重复键（按写入顺序保留多值）；
/// 键值分隔符由选项决定（`=` 紧凑 / ` = ` 疏朗）
fn json_to_ini_string(
    value: &Value,
    options: &ConvertOptions,
) -> Result<String, (ErrorCode, ParseFailure)> {
    let Value::Object(obj) = value else {
        return Err(ser_fail(
            ErrorCode::NonTableRoot,
            format!(
                "{} requires a top-level object",
                format_display_name(ConvertFormat::Ini)
            ),
        ));
    };
    let mut ini = ini::Ini::new();
    for (key, item) in obj {
        match item {
            Value::Object(section) => {
                let mut setter = ini.with_section(Some(key.clone()));
                for (name, field) in section {
                    match field {
                        Value::Array(items) => {
                            for item in items {
                                let text = ini_scalar_to_string(item).map_err(|message| {
                                    ser_fail(
                                        ErrorCode::UnsupportedValue,
                                        format!("section [{key}] field {name}: {message}"),
                                    )
                                })?;
                                setter.add(name, text);
                            }
                        }
                        scalar => {
                            let text = ini_scalar_to_string(scalar).map_err(|message| {
                                ser_fail(
                                    ErrorCode::UnsupportedValue,
                                    format!("section [{key}] field {name}: {message}"),
                                )
                            })?;
                            setter.set(name, text);
                        }
                    }
                }
            }
            Value::Array(items) => {
                let mut setter = ini.with_section(None::<String>);
                for item in items {
                    let text = ini_scalar_to_string(item)
                        .map_err(|message| ser_fail(ErrorCode::UnsupportedValue, message))?;
                    setter.add(key, text);
                }
            }
            scalar => {
                let text = ini_scalar_to_string(scalar)
                    .map_err(|message| ser_fail(ErrorCode::UnsupportedValue, message))?;
                ini.with_section(None::<String>).set(key, text);
            }
        }
    }
    let kv_separator = match options.ini_kv_separator {
        IniKvSeparator::Compact => "=",
        IniKvSeparator::Spaced => " = ",
    };
    let mut buf = Vec::new();
    ini.write_to_opt(
        &mut buf,
        ini::WriteOption {
            kv_separator,
            ..Default::default()
        },
    )
    .map_err(|err| ser_fail(ErrorCode::ParseFailed, err.to_string()))?;
    String::from_utf8(buf).map_err(|err| ser_fail(ErrorCode::ParseFailed, err.to_string()))
}

/// INI 标量转字符串：数字布尔直出，`null`/数组/对象无 INI 表示
fn ini_scalar_to_string(value: &Value) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => {
            Err("only string/number/boolean scalars have an INI form".to_string())
        }
    }
}

/// Properties 解析为 IR：扁平字符串映射（续行与转义由库处理，重复键后者覆盖）
fn parse_properties(input: &str) -> Result<Value, ParseFailure> {
    // `line_number` 已是 1 基，列信息库不提供
    let map: std::collections::HashMap<String, String> = java_properties::read(input.as_bytes())
        .map_err(|err| {
            err.line_number().map_or_else(
                || ParseFailure::new(err.to_string()),
                |line| ParseFailure::at(err.to_string(), line, None),
            )
        })?;
    Ok(Value::Object(
        map.into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect(),
    ))
}

/// IR 转 Properties 文本：顶层必须为对象且只能一层扁平键；标量转字符串，余者报错。
/// 输出键按字典序排列（稳定可 diff）；编码走 UTF-8 直写（库默认 `WINDOWS_1252` 会把
/// 非 Latin-1 字符降级成 `&#N;` 数字引用，既不可读也不能回读）；
/// `properties_escape_unicode` 开启时非 ASCII 字符转写成 `\uXXXX`（经典 Java 风格，ASCII 安全）
fn json_to_properties_string(
    value: &Value,
    options: &ConvertOptions,
) -> Result<String, (ErrorCode, ParseFailure)> {
    let Value::Object(obj) = value else {
        return Err(ser_fail(
            ErrorCode::NonTableRoot,
            format!(
                "{} requires a top-level object",
                format_display_name(ConvertFormat::Properties)
            ),
        ));
    };
    let mut map = std::collections::BTreeMap::new();
    for (key, item) in obj {
        match item {
            Value::String(text) => {
                map.insert(key.clone(), text.clone());
            }
            Value::Number(_) | Value::Bool(_) => {
                map.insert(key.clone(), item.to_string());
            }
            Value::Null | Value::Array(_) | Value::Object(_) => {
                return Err(ser_fail(
                    ErrorCode::UnsupportedValue,
                    format!("key {key}: only string/number/boolean scalars have a Properties form"),
                ));
            }
        }
    }
    let mut buf = Vec::new();
    let mut writer =
        java_properties::PropertiesWriter::new_with_encoding(&mut buf, encoding_rs::UTF_8);
    for (key, text) in &map {
        writer
            .write(key, text)
            .map_err(|err| ser_fail(ErrorCode::ParseFailed, err.to_string()))?;
    }
    writer
        .finish()
        .map_err(|err| ser_fail(ErrorCode::ParseFailed, err.to_string()))?;
    let text =
        String::from_utf8(buf).map_err(|err| ser_fail(ErrorCode::ParseFailed, err.to_string()))?;
    if options.properties_escape_unicode {
        Ok(escape_non_ascii(&text))
    } else {
        Ok(text)
    }
}

/// 非 ASCII 字符转写成 `\uXXXX`（超平面字符拆代理对）；
/// ASCII（含库已写出的 `\ ` 等转义）原样保留，不二次转义反斜杠
fn escape_non_ascii(text: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii() {
            out.push(ch);
        } else {
            let code = ch as u32;
            if code <= 0xFFFF {
                // `String` 写入永不失败；`let _` 显式忽略 `fmt::Result`
                let _ = write!(out, "\\u{code:04X}");
            } else {
                let shifted = code - 0x1_0000;
                let _ = write!(
                    out,
                    "\\u{:04X}\\u{:04X}",
                    0xD800 + (shifted >> 10),
                    0xDC00 + (shifted & 0x3FF)
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> ConvertOptions {
        ConvertOptions {
            json_indent: JsonIndent::Two,
            xml_root_name: "root".to_string(),
            xml_declaration: false,
            xml_indent: JsonIndent::Two,
            ini_kv_separator: IniKvSeparator::Compact,
            properties_escape_unicode: true,
            xml_trailing_newline: true,
        }
    }

    #[test]
    fn detect_prefers_json_over_yaml_superset() {
        // JSON 文本同样是合法 YAML，严格解析必须先截获
        assert_eq!(detect_format(r#"{"a": 1}"#), Some(ConvertFormat::Json));
        assert_eq!(detect_format("a: 1"), Some(ConvertFormat::Yaml));
    }

    #[test]
    fn detect_orders_toml_before_yaml() {
        // `k = v` 在 YAML 里只是普通字符串，必须先按 TOML 试
        assert_eq!(detect_format("title = \"hi\""), Some(ConvertFormat::Toml));
        assert_eq!(detect_format("<root/>"), Some(ConvertFormat::Xml));
        assert_eq!(detect_format("{{{"), None);
        assert_eq!(detect_format("   "), None);
    }

    #[test]
    fn convert_json_to_yaml_and_back() {
        let outcome = convert(
            r#"{"a": 1, "b": [true, null]}"#,
            None,
            ConvertFormat::Yaml,
            &default_options(),
        );
        assert!(outcome.ok);
        assert_eq!(outcome.detected, Some(ConvertFormat::Json));
        let back = convert(
            &outcome.output,
            None,
            ConvertFormat::Json,
            &default_options(),
        );
        assert!(back.ok);
        assert_eq!(back.detected, Some(ConvertFormat::Yaml));
        let value: Value = serde_json::from_str(&back.output).unwrap();
        assert_eq!(value["a"], 1);
    }

    #[test]
    fn convert_toml_roundtrip() {
        let outcome = convert(
            "title = \"hi\"\n[nest]\ncount = 2",
            None,
            ConvertFormat::Json,
            &default_options(),
        );
        assert!(outcome.ok);
        assert_eq!(outcome.detected, Some(ConvertFormat::Toml));
        assert!(outcome.output.contains("\"title\": \"hi\""));
    }

    #[test]
    fn convert_same_format_applies_indent_option() {
        let four = ConvertOptions {
            json_indent: JsonIndent::Four,
            ..default_options()
        };
        let outcome = convert(
            r#"{"a": 1}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Json,
            &four,
        );
        assert!(outcome.ok);
        assert!(outcome.output.contains("\n    \"a\""));
    }

    #[test]
    fn convert_xml_output_escapes_names_root_and_indents() {
        let options = ConvertOptions {
            xml_root_name: "9bad name!".to_string(),
            xml_declaration: true,
            ..default_options()
        };
        let outcome = convert(
            r#"{"msg": "a&b <c>", "n": 1}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Xml,
            &options,
        );
        assert!(outcome.ok);
        assert_eq!(
            outcome.output,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n  <msg>a&amp;b &lt;c&gt;</msg>\n  <n>1</n>\n</root>\n"
        );
    }

    #[test]
    fn convert_xml_output_honors_indent_option() {
        let options = ConvertOptions {
            xml_indent: JsonIndent::Tab,
            ..default_options()
        };
        let outcome = convert(
            r#"{"a": {"b": 1}}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Xml,
            &options,
        );
        assert!(outcome.ok);
        assert!(
            outcome
                .output
                .contains("<root>\n\t<a>\n\t\t<b>1</b>\n\t</a>\n</root>\n")
        );
    }

    #[test]
    fn convert_reports_parse_failure_with_format() {
        let outcome = convert(
            "{bad",
            Some(ConvertFormat::Json),
            ConvertFormat::Yaml,
            &default_options(),
        );
        assert!(!outcome.ok);
        let error = outcome.error.unwrap();
        assert_eq!(error.code, ErrorCode::ParseFailed);
        assert_eq!(error.format, Some(ConvertFormat::Json));
        assert!(error.message.is_some());
    }

    #[test]
    fn convert_xml_input_to_json() {
        let outcome = convert(
            r#"<?xml version="1.0"?><!-- hi --><root><a>1</a><a>2</a><b x="y">t</b><e/></root>"#,
            None,
            ConvertFormat::Json,
            &default_options(),
        );
        assert!(outcome.ok);
        assert_eq!(outcome.detected, Some(ConvertFormat::Xml));
        let value: Value = serde_json::from_str(&outcome.output).unwrap();
        assert_eq!(value["a"], serde_json::json!(["1", "2"]));
        assert_eq!(value["b"]["@x"], "y");
        assert_eq!(value["b"]["#text"], "t");
        assert_eq!(value["e"], Value::Null);
    }

    #[test]
    fn convert_xml_rejects_mixed_content_and_mismatched_tags() {
        let mixed = convert(
            "<a>hi<b>x</b></a>",
            Some(ConvertFormat::Xml),
            ConvertFormat::Json,
            &default_options(),
        );
        assert_eq!(mixed.error.unwrap().code, ErrorCode::ParseFailed);

        let mismatched = convert(
            "<a></b>",
            Some(ConvertFormat::Xml),
            ConvertFormat::Json,
            &default_options(),
        );
        assert_eq!(mismatched.error.unwrap().code, ErrorCode::ParseFailed);
    }

    #[test]
    fn convert_ini_roundtrip_with_sections_and_repeats() {
        let outcome = convert(
            "[server]\nhost = localhost\nport = abc\ntag = x\ntag = y",
            None,
            ConvertFormat::Json,
            &default_options(),
        );
        assert!(outcome.ok);
        assert_eq!(outcome.detected, Some(ConvertFormat::Ini));
        let value: Value = serde_json::from_str(&outcome.output).unwrap();
        assert_eq!(value["server"]["host"], "localhost");
        assert_eq!(value["server"]["tag"], serde_json::json!(["x", "y"]));

        let back = convert(
            &outcome.output,
            None,
            ConvertFormat::Ini,
            &default_options(),
        );
        assert!(back.ok);
        assert!(back.output.contains("[server]"));
        // `rust-ini` 默认键值分隔符为 `=`（无空格），断言按实际输出写
        assert!(back.output.contains("tag=x"));
        assert!(back.output.contains("tag=y"));
    }

    #[test]
    fn convert_ini_rejects_deep_nesting_and_non_table() {
        let deep = convert(
            r#"{"s": {"a": {"b": 1}}}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Ini,
            &default_options(),
        );
        assert_eq!(deep.error.unwrap().code, ErrorCode::UnsupportedValue);

        let array = convert(
            "[1]",
            Some(ConvertFormat::Json),
            ConvertFormat::Ini,
            &default_options(),
        );
        assert_eq!(array.error.unwrap().code, ErrorCode::NonTableRoot);
    }

    #[test]
    fn convert_properties_roundtrip_with_escapes() {
        let outcome = convert(
            "greeting = hello world\npath = C:\\\\temp\nemoji = \\u4e2d",
            None,
            ConvertFormat::Json,
            &default_options(),
        );
        assert!(outcome.ok);
        let value: Value = serde_json::from_str(&outcome.output).unwrap();
        assert_eq!(value["greeting"], "hello world");

        let back = convert(
            &outcome.output,
            None,
            ConvertFormat::Properties,
            &default_options(),
        );
        assert!(back.ok);
    }

    #[test]
    fn convert_properties_rejects_nested_and_non_table() {
        let nested = convert(
            r#"{"a": {"b": 1}}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Properties,
            &default_options(),
        );
        assert_eq!(nested.error.unwrap().code, ErrorCode::UnsupportedValue);

        let scalar = convert(
            "1",
            Some(ConvertFormat::Json),
            ConvertFormat::Properties,
            &default_options(),
        );
        assert_eq!(scalar.error.unwrap().code, ErrorCode::NonTableRoot);
    }

    #[test]
    fn detect_finds_ini_and_properties() {
        assert_eq!(
            detect_format("[s]\nkey = unquoted value"),
            Some(ConvertFormat::Ini)
        );
        assert_eq!(
            detect_format("a = 1\n} stray"),
            Some(ConvertFormat::Properties)
        );
        // 简单 k=v 仍判 TOML（两者解释一致）；`{{{` 无分隔符，维持未知
        assert_eq!(detect_format("a = 1"), Some(ConvertFormat::Toml));
    }

    #[test]
    fn convert_rejects_non_table_toml_root_and_null() {
        let array = convert(
            "[1, 2]",
            Some(ConvertFormat::Json),
            ConvertFormat::Toml,
            &default_options(),
        );
        assert_eq!(array.error.unwrap().code, ErrorCode::NonTableRoot);

        let null = convert(
            r#"{"a": null}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Toml,
            &default_options(),
        );
        assert_eq!(null.error.unwrap().code, ErrorCode::UnsupportedValue);
    }

    #[test]
    fn convert_rejects_oversize_and_clears_empty() {
        let big = "x".repeat(1024 * 1024 + 1);
        let outcome = convert(&big, None, ConvertFormat::Json, &default_options());
        assert_eq!(outcome.error.unwrap().code, ErrorCode::TooLarge);

        let empty = convert("  \n ", None, ConvertFormat::Json, &default_options());
        assert!(empty.ok);
        assert_eq!(empty.output, "");
    }

    #[test]
    fn convert_reports_unknown_format() {
        let outcome = convert("{{{", None, ConvertFormat::Json, &default_options());
        assert_eq!(outcome.error.unwrap().code, ErrorCode::UnknownFormat);
    }

    #[test]
    fn convert_reports_error_lines_per_format() {
        // JSON：第二行缺值，行号精确到 2
        let json = convert(
            "{\n\"a\": }",
            Some(ConvertFormat::Json),
            ConvertFormat::Yaml,
            &default_options(),
        );
        let error = json.error.unwrap();
        assert_eq!(error.code, ErrorCode::ParseFailed);
        assert_eq!(error.line, Some(2));
        assert!(error.column.is_some());

        // YAML：第二行流序列未闭合
        let yaml = convert(
            "a: [1,\n b",
            Some(ConvertFormat::Yaml),
            ConvertFormat::Json,
            &default_options(),
        );
        let error = yaml.error.unwrap();
        assert_eq!(error.code, ErrorCode::ParseFailed);
        assert_eq!(error.line, Some(2));

        // TOML：第一行表头未闭合
        let toml = convert(
            "[a\nb = 1",
            Some(ConvertFormat::Toml),
            ConvertFormat::Json,
            &default_options(),
        );
        let error = toml.error.unwrap();
        assert_eq!(error.code, ErrorCode::ParseFailed);
        assert!(error.line.is_some());

        // XML：第二行标签 mismatch
        let xml = convert(
            "<a>\n</b>",
            Some(ConvertFormat::Xml),
            ConvertFormat::Json,
            &default_options(),
        );
        let error = xml.error.unwrap();
        assert_eq!(error.code, ErrorCode::ParseFailed);
        assert_eq!(error.line, Some(2));

        // INI：首行 section 头未闭合（行号按库约定透传，只断言有值）
        let ini = convert(
            "[s",
            Some(ConvertFormat::Ini),
            ConvertFormat::Json,
            &default_options(),
        );
        let error = ini.error.unwrap();
        assert_eq!(error.code, ErrorCode::ParseFailed);
        assert!(error.line.is_some());

        // Properties：非法 unicode 转义
        let props = convert(
            "k = \\uZZZZ",
            Some(ConvertFormat::Properties),
            ConvertFormat::Json,
            &default_options(),
        );
        let error = props.error.unwrap();
        assert_eq!(error.code, ErrorCode::ParseFailed);
        assert_eq!(error.line, Some(1));
    }

    #[test]
    fn convert_ini_separator_option() {
        let spaced = ConvertOptions {
            ini_kv_separator: IniKvSeparator::Spaced,
            ..default_options()
        };
        let outcome = convert(
            r#"{"s": {"a": "1"}}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Ini,
            &spaced,
        );
        assert!(outcome.ok);
        assert!(outcome.output.contains("a = 1"));

        // 默认紧凑：无空格分隔
        let compact = convert(
            r#"{"s": {"a": "1"}}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Ini,
            &default_options(),
        );
        assert!(compact.ok);
        assert!(compact.output.contains("a=1"));
    }

    #[test]
    fn convert_properties_escape_option_roundtrips() {
        // 默认转义：非 ASCII 写成 \uXXXX（ASCII 安全，可回读）
        let escaped = convert(
            r#"{"emoji": "中"}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Properties,
            &default_options(),
        );
        assert!(escaped.ok);
        assert!(escaped.output.contains("\\u4E2D"));
        let back = convert(
            &escaped.output,
            None,
            ConvertFormat::Json,
            &default_options(),
        );
        assert!(back.ok);
        assert!(back.output.contains('中'));

        // 关闭转义：UTF-8 直写
        let raw = ConvertOptions {
            properties_escape_unicode: false,
            ..default_options()
        };
        let outcome = convert(
            r#"{"emoji": "中"}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Properties,
            &raw,
        );
        assert!(outcome.ok);
        assert!(outcome.output.contains('中'));
    }

    #[test]
    fn convert_xml_trailing_newline_option() {
        let bare = ConvertOptions {
            xml_trailing_newline: false,
            ..default_options()
        };
        let outcome = convert(
            r#"{"a": 1}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Xml,
            &bare,
        );
        assert!(outcome.ok);
        assert!(outcome.output.ends_with("</root>"));

        // 默认保留尾换行
        let tailed = convert(
            r#"{"a": 1}"#,
            Some(ConvertFormat::Json),
            ConvertFormat::Xml,
            &default_options(),
        );
        assert!(tailed.ok);
        assert!(tailed.output.ends_with("</root>\n"));
    }
}
