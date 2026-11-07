struct Empty {};
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Empty value = Empty();
    result[0] = 1.0;
}
