use crate::types::{
    Attribute, Element, Enum, EnumKind, EnumVal, Field, FileExtension,
    FileIdentifier, FloatingConstant, Ident, Include, IntegerConstant,
    Metadata, Namespace, Object, ProductKind, ProductType, Root, Rpc,
    RpcMethod, Scalar, Schema, SingleValue, StringConstant, Type, Value,
};
use hexf_parse::parse_hexf64;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while, take_while_m_n},
    character::complete::{
        char, digit0, digit1, hex_digit0, hex_digit1, line_ending, multispace0,
        multispace1, none_of, one_of, space0, space1,
    },
    combinator::{all_consuming, map, map_res, opt, recognize},
    multi::{many0, many1, separated_list, separated_nonempty_list},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};
use std::collections::HashMap;

fn plus_or_minus(input: &str) -> IResult<&str, &str> {
    alt((tag("-"), tag("+")))(input)
}

#[test]
fn test_plus_or_minus() {
    let res = plus_or_minus("+");
    assert_eq!(res, Ok(("", "+")));
    let res = plus_or_minus("-");
    assert_eq!(res, Ok(("", "-")));
    let res = plus_or_minus("/");
    assert!(res.is_err());
}

fn ident(input: &str) -> IResult<&str, Ident> {
    map(
        recognize(preceded(
            take_while_m_n(1, 1, |c: char| c.is_alphabetic() || c == '_'),
            take_while(|c: char| c.is_alphanumeric() || c == '_'),
        )),
        |name: &str| name.to_string(),
    )(input)
}

#[test]
fn test_simple_ident() {
    let result = ident("foo");
    assert_eq!(result, Ok(("", "foo".to_string())));
}

#[test]
fn test_underscore_prefix() {
    let result = ident("_foo");
    assert_eq!(result, Ok(("", "_foo".to_string())));
}

#[test]
fn test_just_underscore() {
    let result = ident("_");
    assert_eq!(result, Ok(("", "_".to_string())));
}

#[test]
fn test_id_with_number() {
    let result = ident("foo1");
    assert_eq!(result, Ok(("", "foo1".to_string())));
}

#[test]
fn test_invalid_ident_contains_valid() {
    let result = ident("1foo");
    assert!(result.is_err());
}

#[test]
fn test_just_number_is_invalid() {
    let result = ident("1");
    assert!(result.is_err());
}

#[test]
fn test_empty_ident() {
    let result = ident("");
    assert!(result.is_err());
}

fn string_constant(input: &str) -> IResult<&str, StringConstant> {
    map(
        delimited(
            tag("\""),
            opt(escaped(none_of("\\\""), '\\', alt((tag("\\"), tag("\""))))),
            tag("\""),
        ),
        |string| string.unwrap_or("").to_string(),
    )(input)
}

fn schema(input: &str) -> IResult<&str, Schema> {
    map(
        tuple((
            many0(delimited(many0(line_ending), include, many1(line_ending))),
            many0(delimited(
                many0(line_ending),
                alt((
                    map(namespace_decl, Element::Namespace),
                    map(type_decl, Element::ProductType),
                    map(enum_decl, Element::Enum),
                    map(root_decl, Element::Root),
                    map(file_extension_decl, Element::FileExtension),
                    map(file_identifier_decl, Element::FileIdentifier),
                    map(attribute_decl, Element::Attribute),
                    map(rpc_decl, Element::Rpc),
                    map(object, Element::Object),
                )),
                many1(line_ending),
            )),
        )),
        |(includes, body)| Schema { includes, body },
    )(input)
}

fn include(input: &str) -> IResult<&str, Include> {
    map(
        delimited(
            tag("include"),
            delimited(space1, string_constant, space0),
            semicolon,
        ),
        Include,
    )(input)
}

#[test]
fn test_include() {
    let result = include("include \"foo\";");
    assert_eq!(result, Ok(("", Include("foo".to_string()))));
}

#[test]
fn test_include_prefix_whitespace() {
    let result = include("include     \"foo\";");
    assert_eq!(result, Ok(("", Include("foo".to_string()))));
}

#[test]
fn test_include_no_prefix_whitespace() {
    let result = include("include\"foo\";");
    assert!(result.is_err());
}

#[test]
fn test_include_trailing_whitespace() {
    let result = include("include \"foo\"    ;");
    assert_eq!(result, Ok(("", Include("foo".to_string()))));
}

fn semicolon(input: &str) -> IResult<&str, &str> {
    tag(";")(input)
}

fn namespace_decl(input: &str) -> IResult<&str, Namespace> {
    map(
        delimited(
            tag("namespace"),
            delimited(
                multispace1,
                separated_nonempty_list(tag("."), ident),
                multispace0,
            ),
            semicolon,
        ),
        Namespace,
    )(input)
}

#[test]
fn test_simple_namespace_decl() {
    let result = namespace_decl("namespace a.b;");
    assert_eq!(
        result,
        Ok(("", Namespace(vec!["a".to_string(), "b".to_string()])))
    );
}

#[test]
fn test_nested_namespace_decl() {
    let result = namespace_decl("namespace a.b.c;");
    assert_eq!(
        result,
        Ok((
            "",
            Namespace(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        ))
    );
}

fn attribute_decl(input: &str) -> IResult<&str, Attribute> {
    map(
        delimited(
            tag("attribute"),
            delimited(multispace1, alt((ident, string_constant)), multispace0),
            semicolon,
        ),
        Attribute,
    )(input)
}

#[test]
fn test_simple_attribute_decl() {
    let result = attribute_decl("attribute a;");
    assert_eq!(result, Ok(("", Attribute("a".to_string()))));
}

#[test]
fn test_quoted_attribute_decl() {
    let result = attribute_decl("attribute \"a\";");
    assert_eq!(result, Ok(("", Attribute("a".to_string()))));
}

fn left_paren(input: &str) -> IResult<&str, &str> {
    tag("(")(input)
}

fn right_paren(input: &str) -> IResult<&str, &str> {
    tag(")")(input)
}

fn left_brace(input: &str) -> IResult<&str, &str> {
    tag("{")(input)
}

fn right_brace(input: &str) -> IResult<&str, &str> {
    tag("}")(input)
}

fn enum_decl(input: &str) -> IResult<&str, Enum> {
    map(
        tuple((
            alt((
                map(
                    tuple((
                        preceded(tag("enum"), ident),
                        preceded(tag(":"), ty),
                    )),
                    |(id, t)| (id, EnumKind::Enum(t)),
                ),
                map(preceded(tag("union"), ident), |id| (id, EnumKind::Union)),
            )),
            metadata,
            delimited(
                left_brace,
                separated_nonempty_list(tag(","), enumval_decl),
                right_brace,
            ),
        )),
        |((ident, kind), metadata, values)| Enum {
            kind,
            metadata,
            values,
            ident,
        },
    )(input)
}

fn root_decl(input: &str) -> IResult<&str, Root> {
    // TODO: Use type scope mapping here or in codegen?
    map(
        delimited(
            tag("root_type"),
            delimited(space1, ident, space0),
            semicolon,
        ),
        Root,
    )(input)
}

#[test]
fn test_root_decl() {
    let res = root_decl("root_type Foo;");
    assert_eq!(res, Ok(("", Root("Foo".to_string()))));
}

fn field_decl(input: &str) -> IResult<&str, Field> {
    map(
        tuple((
            terminated(ident, space0),
            preceded(tag(":"), preceded(space0, ty)),
            opt(preceded(tag("="), scalar)),
            terminated(metadata, semicolon),
        )),
        |(name, ty, scalar, metadata)| Field {
            name,
            ty,
            scalar,
            metadata,
        },
    )(input)
}

fn rpc_decl(input: &str) -> IResult<&str, Rpc> {
    map(
        tuple((
            preceded(tag("rpc_service"), ident),
            delimited(
                delimited(multispace1, left_brace, multispace0),
                many1(rpc_method),
                delimited(multispace1, right_brace, multispace0),
            ),
        )),
        |(name, methods)| Rpc { name, methods },
    )(input)
}

fn rpc_method(input: &str) -> IResult<&str, RpcMethod> {
    map(
        terminated(
            tuple((
                terminated(ident, space1),
                delimited(
                    left_paren,
                    delimited(space0, ident, space0),
                    right_paren,
                ),
                preceded(
                    delimited(space0, tag(":"), space0),
                    tuple((ident, preceded(space1, metadata))),
                ),
            )),
            preceded(space0, semicolon),
        ),
        |(name, request_type, (response_type, metadata))| RpcMethod {
            name,
            request_type,
            response_type,
            metadata,
        },
    )(input)
}

fn ty(input: &str) -> IResult<&str, Type> {
    alt((
        alt((
            map(tag("bool"), |_| (Type::Bool)),
            map(tag("byte"), |_| (Type::Byte)),
            map(tag("ubyte"), |_| (Type::UByte)),
            map(tag("short"), |_| (Type::Short)),
            map(tag("ushort"), |_| (Type::UShort)),
            map(tag("int"), |_| (Type::Int)),
            map(tag("uint"), |_| (Type::UInt)),
            map(tag("float"), |_| (Type::Float)),
            map(tag("long"), |_| (Type::Long)),
            map(tag("ulong"), |_| (Type::ULong)),
            map(tag("double"), |_| (Type::Double)),
            map(tag("int8"), |_| (Type::Int8)),
            map(tag("uint8"), |_| (Type::UInt8)),
            map(tag("int16"), |_| (Type::Int16)),
            map(tag("uint16"), |_| (Type::UInt16)),
            map(tag("int32"), |_| (Type::Int32)),
            map(tag("uint32"), |_| (Type::UInt32)),
            map(tag("int64"), |_| (Type::Int64)),
            map(tag("uint64"), |_| (Type::UInt64)),
            map(tag("float32"), |_| (Type::Float32)),
            map(tag("float64"), |_| (Type::Float64)),
        )),
        map(tag("string"), |_| Type::String),
        map(delimited(tag("["), ty, tag("]")), |t| {
            Type::Array(Box::new(t))
        }),
        map(ident, Type::Ident),
    ))(input)
}

fn enumval_decl(input: &str) -> IResult<&str, EnumVal> {
    map(tuple((ident, opt(integer_constant))), |(name, value)| {
        EnumVal { name, value }
    })(input)
}

fn metadata(input: &str) -> IResult<&str, Option<Metadata>> {
    opt(map(
        delimited(
            tag("("),
            separated_list(
                tag(","),
                separated_pair(ident, tag(":"), opt(single_value)),
            ),
            tag(")"),
        ),
        |values| {
            let mut res = HashMap::new();
            for (key, value) in values {
                res.insert(key, value);
            }
            Metadata(res)
        },
    ))(input)
}

fn scalar(input: &str) -> IResult<&str, Scalar> {
    alt((
        map(float_constant, Scalar::Float),
        map(integer_constant, Scalar::Integer),
    ))(input)
}

fn object(input: &str) -> IResult<&str, Object> {
    map(
        delimited(
            tag("{"),
            separated_nonempty_list(
                tag(","),
                separated_pair(ident, tag(":"), value),
            ),
            tag("}"),
        ),
        |values| {
            let mut res = HashMap::new();
            for (key, value) in values {
                res.insert(key, value);
            }
            Object(res)
        },
    )(input)
}

fn single_value(input: &str) -> IResult<&str, SingleValue> {
    alt((
        map(scalar, SingleValue::Scalar),
        map(string_constant, SingleValue::StringConstant),
    ))(input)
}

fn value(input: &str) -> IResult<&str, Value> {
    alt((
        map(single_value, Value::SingleValue),
        map(object, Value::Object),
    ))(input)
}

fn type_decl(input: &str) -> IResult<&str, ProductType> {
    map(
        tuple((
            alt((
                map(tag("table"), |_| ProductKind::Table),
                map(tag("struct"), |_| ProductKind::Struct),
            )),
            delimited(space1, ident, space0),
            metadata,
            delimited(
                left_brace,
                delimited(multispace0, many1(field_decl), multispace0),
                right_brace,
            ),
        )),
        |(kind, name, metadata, fields)| ProductType {
            kind,
            name,
            fields,
            metadata,
        },
    )(input)
}

#[test]
fn test_product_type() {
    let table = "table HelloReply { message: string; }";
    let res = type_decl(table);
    assert_eq!(
        res,
        Ok((
            "",
            ProductType {
                kind: ProductKind::Table,
                name: "HelloReply".to_string(),
                fields: vec![Field {
                    name: "message".to_string(),
                    ty: Type::String,
                    scalar: None,
                    metadata: None
                }],
                metadata: None
            }
        ))
    );
}

fn dec_integer_constant(input: &str) -> IResult<&str, IntegerConstant> {
    map_res(
        all_consuming(recognize(preceded(opt(plus_or_minus), digit1))),
        |value| IntegerConstant::from_str_radix(value, 10),
    )(input)
}

#[test]
fn test_dec_integer_constant() {
    let res = dec_integer_constant("1234");
    assert_eq!(res, Ok(("", 1234)));

    let res = dec_integer_constant("-1234");
    assert_eq!(res, Ok(("", -1234)));

    let res = dec_integer_constant("-0x1234");
    assert!(res.is_err());

    let res = dec_integer_constant("0x1234");
    assert!(res.is_err());
}

fn hex_integer_constant(input: &str) -> IResult<&str, IntegerConstant> {
    map(
        all_consuming(tuple((
            opt(plus_or_minus),
            preceded(
                char('0'),
                preceded(
                    one_of("xX"),
                    map_res(hex_digit1, |value| {
                        IntegerConstant::from_str_radix(value, 16)
                    }),
                ),
            ),
        ))),
        |(sign, value)| {
            if let Some("-") = sign {
                -value
            } else {
                value
            }
        },
    )(input)
}

#[test]
fn test_hex_integer_constant() {
    let res = hex_integer_constant("0x1234ABCDEFabcdef");
    assert_eq!(res, Ok(("", 0x1234ABCDEFabcdef)));

    let res = hex_integer_constant("-0x1234ABCDEFabcdef");
    assert_eq!(res, Ok(("", -0x1234ABCDEFabcdef)));

    let res = hex_integer_constant("ABCDEFabcdef");
    assert!(res.is_err());
}

fn true_(input: &str) -> IResult<&str, bool> {
    map(all_consuming(tag("true")), |_| true)(input)
}

#[test]
fn test_true() {
    let res = true_("true");
    assert_eq!(res, Ok(("", true)));

    let res = true_("truez");
    assert!(res.is_err());
}

fn false_(input: &str) -> IResult<&str, bool> {
    map(all_consuming(tag("false")), |_| false)(input)
}

#[test]
fn test_false() {
    let res = false_("false");
    assert_eq!(res, Ok(("", false)));
    let res = false_("falsez");
    assert!(res.is_err());
}

fn boolean_constant(input: &str) -> IResult<&str, bool> {
    alt((true_, false_))(input)
}

#[test]
fn test_boolean_constant() {
    let res = boolean_constant("true");
    assert_eq!(res, Ok(("", true)));
    let res = boolean_constant("false");
    assert_eq!(res, Ok(("", false)));
    let res = boolean_constant("waltz");
    assert!(res.is_err());
}

fn float_constant(input: &str) -> IResult<&str, FloatingConstant> {
    alt((
        special_float_constant,
        hex_float_constant,
        dec_float_constant,
    ))(input)
}

fn dec_float_constant(input: &str) -> IResult<&str, FloatingConstant> {
    map_res(
        all_consuming(recognize(terminated(
            preceded(
                opt(plus_or_minus),
                alt((
                    recognize(preceded(char('.'), digit1)),
                    recognize(delimited(digit1, char('.'), digit0)),
                    digit1,
                )),
            ),
            opt(preceded(
                one_of("eE"),
                recognize(preceded(opt(plus_or_minus), digit1)),
            )),
        ))),
        |number| number.parse::<FloatingConstant>(),
    )(input)
}

#[test]
fn test_dec_float_constant() {
    let res = dec_float_constant("2.0");
    assert_eq!(res, Ok(("", 2.0)));

    let res = dec_float_constant("2.0e5");
    assert_eq!(res, Ok(("", 2.0e5)));

    let res = dec_float_constant("-2.0e5");
    assert_eq!(res, Ok(("", -2.0e5)));

    let res = dec_float_constant("2.1e5");
    assert_eq!(res, Ok(("", 2.1e5)));

    let res = dec_float_constant("-2.1e5");
    assert_eq!(res, Ok(("", -2.1e5)));

    let res = dec_float_constant(".91234e5");
    assert_eq!(res, Ok(("", 0.91234e5)));

    let res = dec_float_constant("-.12e5");
    assert_eq!(res, Ok(("", -0.12e5)));

    let res = dec_float_constant("2.0E5");
    assert_eq!(res, Ok(("", 2.0e5)));

    let res = dec_float_constant("-2.0E5");
    assert_eq!(res, Ok(("", -2.0e5)));

    let res = dec_float_constant("2.1E5");
    assert_eq!(res, Ok(("", 2.1e5)));

    let res = dec_float_constant("-2.1E5");
    assert_eq!(res, Ok(("", -2.1e5)));

    let res = dec_float_constant(".91234E5");
    assert_eq!(res, Ok(("", 0.91234e5)));

    let res = dec_float_constant("-.12E5");
    assert_eq!(res, Ok(("", -0.12e5)));
}

fn hex_float_constant(input: &str) -> IResult<&str, FloatingConstant> {
    map_res(
        all_consuming(recognize(preceded(
            opt(plus_or_minus),
            preceded(
                char('0'),
                preceded(
                    one_of("xX"),
                    terminated(
                        alt((
                            recognize(preceded(char('.'), hex_digit1)),
                            recognize(delimited(
                                hex_digit1,
                                char('.'),
                                hex_digit0,
                            )),
                            hex_digit1,
                        )),
                        preceded(
                            one_of("pP"),
                            recognize(preceded(opt(plus_or_minus), digit1)),
                        ),
                    ),
                ),
            ),
        ))),
        |value| parse_hexf64(value, false),
    )(input)
}

fn nan(input: &str) -> IResult<&str, FloatingConstant> {
    map(all_consuming(tag("nan")), |_| std::f64::NAN)(input)
}

#[test]
fn test_nan() {
    let res = nan("nan");
    assert_eq!(
        res.map(|(input, value)| input.is_empty() && value.is_nan()),
        Ok(true)
    );
    let res = nan("infinity");
    assert!(res.is_err());
    let res = nan("nanz");
    assert!(res.is_err());
}

fn inf(input: &str) -> IResult<&str, FloatingConstant> {
    map(all_consuming(tag("inf")), |_| std::f64::INFINITY)(input)
}

#[test]
fn test_inf() {
    let res = inf("inf");
    assert_eq!(res, Ok(("", std::f64::INFINITY)));
    let res = inf("infinity");
    assert!(res.is_err());
}

fn infinity(input: &str) -> IResult<&str, FloatingConstant> {
    map(all_consuming(tag("infinity")), |_| std::f64::INFINITY)(input)
}

#[test]
fn test_infinity() {
    let res = infinity("infinity");
    assert_eq!(res, Ok(("", std::f64::INFINITY)));
    let res = infinity("infinitys");
    assert!(res.is_err());
    let res = infinity("foo");
    assert!(res.is_err());
}

fn special_float_constant(input: &str) -> IResult<&str, FloatingConstant> {
    map(
        tuple((opt(plus_or_minus), alt((nan, infinity, inf)))),
        |(sign, value)| {
            if let Some("-") = sign {
                -value
            } else {
                value
            }
        },
    )(input)
}

#[test]
fn test_special_float_constant_nan() {
    let res = special_float_constant("nan");
    assert_eq!(
        res.map(|(input, value)| input.is_empty()
            && value.is_nan()
            && value.is_sign_positive()),
        Ok(true)
    );
    let res = special_float_constant("-nan");
    assert_eq!(
        res.map(|(input, value)| input.is_empty()
            && value.is_nan()
            && value.is_sign_negative()),
        Ok(true)
    );
}

#[test]
fn test_special_float_constant_inf() {
    let res = special_float_constant("inf");
    assert_eq!(res, Ok(("", std::f64::INFINITY)));
    let res = special_float_constant("-inf");
    assert_eq!(res, Ok(("", std::f64::NEG_INFINITY)));
}

#[test]
fn test_special_float_constant_infinity() {
    let res = special_float_constant("infinity");
    assert_eq!(res, Ok(("", std::f64::INFINITY)));
    let res = special_float_constant("-infinity");
    assert_eq!(res, Ok(("", std::f64::NEG_INFINITY)));
}

fn integer_constant(input: &str) -> IResult<&str, IntegerConstant> {
    alt((hex_integer_constant, dec_integer_constant))(input)
}

#[test]
fn test_integer_constant() {
    let res = integer_constant("1234");
    assert_eq!(res, Ok(("", 1234)));

    let res = integer_constant("-1234");
    assert_eq!(res, Ok(("", -1234)));

    let res = integer_constant("0x1234");
    assert_eq!(res, Ok(("", 0x1234)));

    let res = integer_constant("-0x1234");
    assert_eq!(res, Ok(("", -0x1234)));
}

fn file_extension_decl(input: &str) -> IResult<&str, FileExtension> {
    delimited(
        tag("file_extension"),
        delimited(space0, string_constant, space0),
        semicolon,
    )(input)
}

#[test]
fn test_file_extension_decl() {
    let res = file_extension_decl("file_extension \"foo\";");
    assert_eq!(res, Ok(("", "foo".to_string())));
}

#[test]
fn test_file_extension_decl_no_leading_space() {
    let res = file_extension_decl("file_extension\"foo\";");
    assert_eq!(res, Ok(("", "foo".to_string())));
}

#[test]
fn test_file_extension_decl_trailing_space() {
    let res = file_extension_decl("file_extension \"foo\"  ;");
    assert_eq!(res, Ok(("", "foo".to_string())));
}

#[test]
fn test_file_extension_decl_surrounding_space() {
    let res = file_extension_decl("file_extension   \"foo\"  ;");
    assert_eq!(res, Ok(("", "foo".to_string())));
}

fn file_identifier_decl(input: &str) -> IResult<&str, FileIdentifier> {
    delimited(
        tag("file_identifier"),
        delimited(space0, string_constant, space0),
        semicolon,
    )(input)
}

#[test]
fn test_file_identifier_decl() {
    let res = file_identifier_decl("file_identifier \"foo\";");
    assert_eq!(res, Ok(("", "foo".to_string())));
}

#[test]
fn test_file_identifier_decl_no_leading_space() {
    let res = file_identifier_decl("file_identifier\"foo\";");
    assert_eq!(res, Ok(("", "foo".to_string())));
}

#[test]
fn test_file_identifier_decl_trailing_space() {
    let res = file_identifier_decl("file_identifier \"foo\"  ;");
    assert_eq!(res, Ok(("", "foo".to_string())));
}

#[test]
fn test_file_identifier_decl_surrounding_space() {
    let res = file_identifier_decl("file_identifier   \"foo\"  ;");
    assert_eq!(res, Ok(("", "foo".to_string())));
}