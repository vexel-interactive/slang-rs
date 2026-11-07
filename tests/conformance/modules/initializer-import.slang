import "initializer-lib.slang";
float defaultValue() { return 9.0; }
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Container value = {};
    result[0] = read(value) + value.pair.x;
}
