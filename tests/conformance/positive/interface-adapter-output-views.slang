RWStructuredBuffer<uint> output;
interface SignedImplementation {
    uint replace(float input, out uint value);
    uint shift(float input, inout uint value);
}
struct Signed : SignedImplementation {
    int replace(int input, out int value) { value = -1; return -2; }
    int shift(int input, inout int value) { value >>= input; return value; }
};
uint replace<T : SignedImplementation>(T item, out uint value) { return item.replace(1.9, value); }
uint shift<T : SignedImplementation>(T item, inout uint value) { return item.shift(1.9, value); }
interface UnsignedImplementation {
    int shift(float input, inout int value);
}
struct Unsigned : UnsignedImplementation {
    uint shift(int input, inout uint value) { value >>= input; return value; }
};
int reverseShift<T : UnsignedImplementation>(T item, inout int value) { return item.shift(1.9, value); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Signed signedValue;
    uint replaced;
    output[0] = replace(signedValue, replaced);
    output[1] = replaced;
    uint values[2] = { 0x80000000u, 7u };
    int index = 0;
    output[2] = shift(signedValue, values[index++]);
    output[3] = values[0];
    output[4] = uint(index);
    output[5] = values[1];
    Unsigned unsignedValue;
    int reversed = -1;
    output[6] = uint(reverseShift(unsignedValue, reversed));
    output[7] = uint(reversed);
}
