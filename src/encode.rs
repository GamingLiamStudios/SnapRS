use crate::text::TextComponent;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum EncodeError {
    MalformedVarint,
    StringBounds,
}

impl EncodeError {
    #[must_use]
    pub fn describe(self) -> TextComponent {
        match self {
            Self::MalformedVarint => {
                TextComponent::new_text("Server attempted to send malformed varint")
            },
            Self::StringBounds => {
                TextComponent::new_text("Server attempted to send too large of a string")
            },
        }
    }
}

pub trait Generate<E> {
    /// # Errors
    /// Depends on implementation
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, E>;

    /// # Errors
    /// See [`generate_in_place`](Generate::generate_in_place)
    fn generate(&self) -> Result<Vec<u8>, E> {
        let mut buf = Vec::new();
        let _ = self.generate_in_place(&mut buf)?;
        Ok(buf)
    }
}

pub trait SerializeFn<E>: Fn(&mut Vec<u8>) -> Result<usize, E> {}
impl<E, F: Fn(&mut Vec<u8>) -> Result<usize, E>> SerializeFn<E> for F {}
impl<E, F: SerializeFn<E> + ?Sized> Generate<E> for F {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, E> {
        self(buf)
    }
}

impl<E, T: Generate<E>> Generate<E> for Option<T> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, E> {
        self.as_ref()
            .map_or_else(|| Ok(0), |value| value.generate_in_place(buf))
    }
}

// Removes specialization for &[u8] however :(
impl<E, T: Generate<E>> Generate<E> for &[T] {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, E> {
        let mut running_total = 0;
        for value in *self {
            running_total += value.generate_in_place(buf)?;
        }
        Ok(running_total)
    }
}

pub fn cond<E>(
    value: impl Generate<E>,
    condition: impl Fn() -> bool,
) -> impl SerializeFn<E> {
    move |buf| {
        if condition() {
            value.generate_in_place(buf)
        } else {
            Ok(0)
        }
    }
}

pub fn length_value<E, Fi: Generate<E>, Fo: Fn(usize) -> Fi>(
    value: impl Generate<E>,
    length_writer: Fo,
) -> impl SerializeFn<E> {
    move |buf| {
        let begin = buf.len();
        value.generate_in_place(buf)?;
        let data_length = buf.len() - begin;

        let bytes = length_writer(data_length).generate()?;
        buf.resize(buf.len() + bytes.len(), 0);
        buf.copy_within(begin..begin + data_length, begin + bytes.len());
        buf[begin..begin + bytes.len()].copy_from_slice(&bytes);
        Ok(buf.len() - begin)
    }
}

/// Uses least amount of bytes to write integer
#[must_use]
pub fn write_varint<E: From<EncodeError>>(value: i32) -> impl SerializeFn<E> {
    move |buf| {
        let mut value = value.cast_unsigned();

        let begin = buf.len();
        for _ in 0..5 {
            let nibble = (value & 0x7f) as u8;
            value >>= 7;

            if value != 0 {
                buf.push(0x80 | nibble);
            } else {
                buf.push(nibble);
                return Ok(buf.len() - begin);
            }
        }

        Err(EncodeError::MalformedVarint.into())
    }
}

/// Uses constant 5 bytes to write integer
#[must_use]
pub fn write_varint_fixed<E>(value: i32) -> impl SerializeFn<E> {
    let mut value = value.cast_unsigned();
    let mut nibbles: [u8; 5] = std::array::from_fn(|_| {
        let nibble = (value & 0x7f) as u8;
        value >>= 7;
        nibble | 0x80
    });
    nibbles[4] &= 0x7f;

    move |buf| {
        buf.extend_from_slice(&nibbles);
        Ok(5)
    }
}

/// # Panics
/// Will panic if the specified string exceeds the specified bounds
#[must_use]
pub fn bounded_string<const BOUND: usize, E: From<EncodeError>>(
    value: &str
) -> impl SerializeFn<E> {
    let bytes = value.as_bytes();
    move |buf| {
        if bytes.len() <= BOUND * 3 {
            length_value(bytes, |length| {
                write_varint(
                    i32::try_from(length).expect("Length of Packet was larger than i32::MAX"),
                )
            })
            .generate_in_place(buf)
        } else {
            Err(EncodeError::StringBounds.into())
        }
    }
}

macro_rules! impl_number {
    ($($name: ty)+) => {
        $(
            impl<E> Generate<E> for $name {
                fn generate_in_place(
                    &self,
                    buf: &mut Vec<u8>,
                ) -> Result<usize, E> {
                    let bytes = self.to_be_bytes();
                    buf.extend_from_slice(&bytes);
                    Ok(bytes.len())
                }
            }
        )+
    };
}

impl_number!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128 f32 f64);

macro_rules! impl_tuple {
    ($($name: ident)+) => {
        impl<Err, $($name: Generate<Err>),+> Generate<Err> for ($($name),+) {
            #[allow(non_snake_case)]
            fn generate_in_place(&self, buf: &mut Vec<u8>) -> Result<usize, Err> {
                let mut running_total = 0;
                let ($($name,)+) = self;
                $(running_total += $name.generate_in_place(buf)?;)+
                Ok(running_total)
            }
        }
    };
}

impl_tuple!(A B);
impl_tuple!(A B C);
impl_tuple!(A B C D);
impl_tuple!(A B C D E);
impl_tuple!(A B C D E F);
impl_tuple!(A B C D E F G);
impl_tuple!(A B C D E F G H);
impl_tuple!(A B C D E F G H I);
impl_tuple!(A B C D E F G H I J);
impl_tuple!(A B C D E F G H I J K);
impl_tuple!(A B C D E F G H I J K L);
impl_tuple!(A B C D E F G H I J K L M);
impl_tuple!(A B C D E F G H I J K L M N);
impl_tuple!(A B C D E F G H I J K L M N O);
impl_tuple!(A B C D E F G H I J K L M N O P);
impl_tuple!(A B C D E F G H I J K L M N O P Q);
impl_tuple!(A B C D E F G H I J K L M N O P Q R);
impl_tuple!(A B C D E F G H I J K L M N O P Q R S);
impl_tuple!(A B C D E F G H I J K L M N O P Q R S T);

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::needless_pass_by_value)]
    fn test_generator(
        encoder: impl Generate<EncodeError>,
        expected: &[u8],
    ) {
        let vector = encoder.generate();
        assert_eq!(vector.as_ref().map(|v| &v[..]), Ok(expected));
    }

    #[test]
    pub fn test_serial_varint() {
        test_generator(write_varint(0), &[0x00]);
        test_generator(write_varint(1), &[0x01]);
        test_generator(write_varint(2), &[0x02]);
        test_generator(write_varint(127), &[0x7f]);
        test_generator(write_varint(128), &[0x80, 0x01]);
        test_generator(write_varint(255), &[0xff, 0x01]);
        test_generator(write_varint(25565), &[0xdd, 0xc7, 0x01]);
        test_generator(write_varint(2_097_151), &[0xff, 0xff, 0x7f]);
        test_generator(write_varint(2_147_483_647), &[0xff, 0xff, 0xff, 0xff, 0x07]);
        test_generator(write_varint(-1), &[0xff, 0xff, 0xff, 0xff, 0x0f]);
        test_generator(write_varint(-2_147_483_648), &[
            0x80, 0x80, 0x80, 0x80, 0x08,
        ]);
    }

    #[test]
    pub fn test_serial_varint_fixed() {
        test_generator(write_varint_fixed(0), &[0x80, 0x80, 0x80, 0x80, 0x00]);
        test_generator(write_varint_fixed(1), &[0x81, 0x80, 0x80, 0x80, 0x00]);
        test_generator(write_varint_fixed(2), &[0x82, 0x80, 0x80, 0x80, 0x00]);
        test_generator(write_varint_fixed(127), &[0xff, 0x80, 0x80, 0x80, 0x00]);
        test_generator(write_varint_fixed(128), &[0x80, 0x81, 0x80, 0x80, 0x00]);
        test_generator(write_varint_fixed(255), &[0xff, 0x81, 0x80, 0x80, 0x00]);
        test_generator(write_varint_fixed(25565), &[0xdd, 0xc7, 0x81, 0x80, 0x00]);
        test_generator(write_varint_fixed(2_097_151), &[
            0xff, 0xff, 0xff, 0x80, 0x00,
        ]);
        test_generator(write_varint_fixed(2_147_483_647), &[
            0xff, 0xff, 0xff, 0xff, 0x07,
        ]);
        test_generator(write_varint_fixed(-1), &[0xff, 0xff, 0xff, 0xff, 0x0f]);
        test_generator(write_varint_fixed(-2_147_483_648), &[
            0x80, 0x80, 0x80, 0x80, 0x08,
        ]);
    }

    #[test]
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    pub fn test_length_value() {
        test_generator(length_value(0i8, |v| write_varint(v as i32)), &[0x01, 0x00]);
        // TODO: More tests
    }

    #[test]
    pub fn test_bounded_string() {
        test_generator(bounded_string::<255, _>("Hello World"), b"\x0bHello World");
        let large_string = "A".repeat(128);
        let mut expected_bytes = vec![0x80, 0x01];
        expected_bytes.extend_from_slice(large_string.as_bytes());
        test_generator(
            bounded_string::<255, _>(large_string.as_str()),
            &expected_bytes,
        );
        // TODO: More tests
    }

    #[test]
    pub fn test_tuple() {
        test_generator((0i8, 1i8, -1i8), &[0x00, 0x01, 0xff]);
        // TODO: More tests
    }
}
