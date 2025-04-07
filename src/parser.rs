use nom::{
    IResult,
    Parser,
    bits::{
        bits,
        streaming as bits,
    },
    combinator::{
        map,
        verify,
    },
    multi::{
        length_data,
        many_m_n,
    },
    sequence::{
        pair,
        preceded,
    },
};

pub fn parse_varbits<const P: u8>(data: &[u8]) -> IResult<&[u8], u8> {
    bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(preceded(
        bits::tag(P, 1usize),
        bits::take(7usize),
    ))(data)
}

// TODO: Use more descriptive/appropriate error
#[allow(clippy::cast_possible_wrap)]
pub fn parse_varint(data: &[u8]) -> IResult<&[u8], i32> {
    map(
        pair(many_m_n(0, 4, parse_varbits::<1>), parse_varbits::<0>),
        |(values, delim)| {
            values
                .iter()
                .chain(std::iter::once(&delim))
                .enumerate()
                .fold(0u32, |acc, (i, v)| acc | (u32::from(*v) << (i * 7))) as i32
        },
    )
    .parse(data)
}

#[allow(clippy::cast_possible_wrap)]
pub fn parse_varlong(data: &[u8]) -> IResult<&[u8], i64> {
    map(
        pair(many_m_n(0, 9, parse_varbits::<1>), parse_varbits::<0>),
        |(values, delim)| {
            values
                .iter()
                .chain(std::iter::once(&delim))
                .enumerate()
                .fold(0u64, |acc, (i, v)| acc | (u64::from(*v) << (i * 7))) as i64
        },
    )
    .parse(data)
}

pub fn parse_string<const MAX: i32>(data: &[u8]) -> IResult<&[u8], &str> {
    assert!(MAX <= 32767, "Invalid Maximum Size");
    length_data(verify(parse_varint, |v| *v <= (MAX * 3)).map(i32::cast_unsigned))
        .map_res(|v| str::from_utf8(v))
        .parse(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_parse() {
        assert_eq!(parse_varint(&[0x00]).map(|(_, v)| v), Ok(0));
        assert_eq!(parse_varint(&[0x01]).map(|(_, v)| v), Ok(1));
        assert_eq!(parse_varint(&[0x02]).map(|(_, v)| v), Ok(2));
        assert_eq!(parse_varint(&[0x7f]).map(|(_, v)| v), Ok(127));
        assert_eq!(parse_varint(&[0x80, 0x01]).map(|(_, v)| v), Ok(128));
        assert_eq!(parse_varint(&[0xff, 0x01]).map(|(_, v)| v), Ok(255));
        assert_eq!(parse_varint(&[0xdd, 0xc7, 0x01]).map(|(_, v)| v), Ok(25565));
        assert_eq!(
            parse_varint(&[0xff, 0xff, 0x7f]).map(|(_, v)| v),
            Ok(2_097_151)
        );
        assert_eq!(
            parse_varint(&[0xff, 0xff, 0xff, 0xff, 0x07]).map(|(_, v)| v),
            Ok(2_147_483_647)
        );
        assert_eq!(
            parse_varint(&[0xff, 0xff, 0xff, 0xff, 0x0f]).map(|(_, v)| v),
            Ok(-1)
        );
        assert_eq!(
            parse_varint(&[0x80, 0x80, 0x80, 0x80, 0x08]).map(|(_, v)| v),
            Ok(-2_147_483_648)
        );

        assert_eq!(
            parse_varint(&[0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(nom::Err::Error(nom::error::Error {
                input: [255u8].as_slice(),
                code:  nom::error::ErrorKind::TagBits,
            }))
        );
    }

    #[test]
    fn test_varlong_parse() {
        assert_eq!(parse_varlong(&[0x00]).map(|(_, v)| v), Ok(0));
        assert_eq!(parse_varlong(&[0x01]).map(|(_, v)| v), Ok(1));
        assert_eq!(parse_varlong(&[0x02]).map(|(_, v)| v), Ok(2));
        assert_eq!(parse_varlong(&[0x7f]).map(|(_, v)| v), Ok(127));

        assert_eq!(parse_varlong(&[0x80, 0x01]).map(|(_, v)| v), Ok(128));
        assert_eq!(parse_varlong(&[0xff, 0x01]).map(|(_, v)| v), Ok(255));

        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0x7f]).map(|(_, v)| v),
            Ok(2_097_151)
        );
        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0x07]).map(|(_, v)| v),
            Ok(2_147_483_647)
        );
        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]).map(|(_, v)| v),
            Ok(9_223_372_036_854_775_807)
        );

        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01])
                .map(|(_, v)| v),
            Ok(-1)
        );
        assert_eq!(
            parse_varlong(&[0x80, 0x80, 0x80, 0x80, 0xf8, 0xff, 0xff, 0xff, 0xff, 0x01])
                .map(|(_, v)| v),
            Ok(-2_147_483_648)
        );
        assert_eq!(
            parse_varlong(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01])
                .map(|(_, v)| v),
            Ok(-9_223_372_036_854_775_808)
        );

        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(nom::Err::Error(nom::error::Error {
                input: [255u8].as_slice(),
                code:  nom::error::ErrorKind::TagBits,
            }))
        );
    }
}
