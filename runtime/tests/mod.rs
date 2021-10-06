use codec::{Compact, Encode};

// helper function to check if a given data structure produces
// the expected bytes when encoded
fn assert_encode<T: Encode>(t: T, bytes: &[u8]) {
    let data = Encode::encode(&t);
    assert_eq!(data, bytes);
}

// derive Encode to automatically generate the encoding for the enum
#[derive(Encode)]
enum TestEnum {
    A,
    B,
    C = 10,
}

#[derive(Encode)]
struct TestStruct {
    a: TestEnum,
    b: u32,
    c: TestEnum,
}

#[derive(Encode)]
enum TestEnum2 {
    A(TestEnum, u32, TestEnum),
    B(TestStruct),
}

#[test]
fn test_codec() {
    // check the encode format for integers
    assert_encode(1u32, b"\x01\0\0\0"); // expect 4 little endian bytes
    assert_encode(1u64, b"\x01\0\0\0\0\0\0\0"); // expect 8 little endian bytes

    // check the encode format for booleans
    assert_encode(true, b"\x01");   // 0x01 can also be decoded as an 8-bit unsigned integer, so
    // without knowing the schema upfront, there is no way to know whether 0x01 is true or is a u8
    // integer 1

    assert_encode(false, b"\x00");

    // check the encode format for enums
    assert_encode(TestEnum::A, b"\x00");    // 0
    assert_encode(TestEnum::B, b"\x01");    // 1
    assert_encode(TestEnum::C, b"\x0a");    // 10

    // check the encode format for tuples
    assert_encode((1u32, 2u32), b"\x01\0\0\0\x02\0\0\0");   // the data of a tuple is just concatenated
    // there are no separators

    assert_encode((TestEnum::A, 2u32, TestEnum::C), b"\0\x02\0\0\0\x0a");

    // check the encode format for structs
    assert_encode(TestStruct {
            a: TestEnum::A,
            b: 2u32,
            c: TestEnum::C,
        },
        b"\0\x02\0\0\0\x0a",    // exactly the same as the tuple above
    );

    // more complicated enums
    assert_encode(
        TestEnum2::A(TestEnum::A, 2u32, TestEnum::C),
        b"\0\0\x02\0\0\0\x0a",  // the first byte indicates which TestEnum2 variable it is (A or B) and then the parameters are concatenated
    );
    assert_encode(
        TestEnum2::B(TestStruct {
            a: TestEnum::A,
            b: 2u32,
            c: TestEnum::C,
        }),
        b"\x01\0\x02\0\0\0\x0a",    // 0x01 for B
    );

    // more advanced data structures
    assert_encode(Vec::<u8>::new(), b"\0");
    assert_encode(vec![1u32, 2u32], b"\x08\x01\0\0\0\x02\0\0\0");   // 0x08 indicate that the size is 2; it is encoded using the Compact encoding (2 * 0x04 bytes)
}

// copied from the SCALE codec
// compact encoding:
// 0b00 00 00 00 / 00 00 00 00 / 00 00 00 00 / 00 00 00 00
//   xx xx xx 00															(0 .. 2**6)		(u8)
//   yL yL yL 01 / yH yH yH yL												(2**6 .. 2**14)	(u8, u16)  low LH high
//   zL zL zL 10 / zM zM zM zL / zM zM zM zM / zH zH zH zM					(2**14 .. 2**30)	(u16, u32)  low LMMH high
//   nn nn nn 11 [ / zz zz zz zz ]{4 + n}									(2**30 .. 2**536)	(u32, u64, u128, U256, U512, U520) straight LE-encoded

// Note: we use *LOW BITS* of the LSB in LE encoding to encode the 2 bit key.

