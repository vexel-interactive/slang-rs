import alias_lib;

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Number value = Number(2);
    result[0] = value + data[0];
}
