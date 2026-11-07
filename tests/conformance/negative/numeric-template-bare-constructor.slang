RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = vector(1.0).x;
}
