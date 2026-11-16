struct Narrow
{
    half2 values;
    int16_t signedValue;
    uint16_t unsignedValue;
};

ByteAddressBuffer input;
RWByteAddressBuffer output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Narrow value = input.Load<Narrow>(0);
    output.Store<Narrow>(0, value);
}
