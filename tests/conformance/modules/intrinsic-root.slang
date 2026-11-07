import intrinsic_lib;

float sin(float value)
{
    return value + 5.0;
}

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = sin(1.0) + read(2.0);
}
